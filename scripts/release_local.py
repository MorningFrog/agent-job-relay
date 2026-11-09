#!/usr/bin/env python3
"""Local maintainer publication of an original Woodpecker candidate; no CI impersonation."""
from contextlib import contextmanager
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote, urlsplit
import uuid

sys.dont_write_bytecode = True
import release_publish as publisher
import release_actions as shared
from release_metadata import REPO, PACKAGES, require, sha256
from release_woodpecker import read_json, verify_prepared

OPERATIONS = {'inspect', 'preview', 'stage', 'finalize', 'check'}
CREDENTIAL_ENV = {'GH_TOKEN','GITHUB_TOKEN','GH_ENTERPRISE_TOKEN','GITHUB_ENTERPRISE_TOKEN',
                  'NODE_AUTH_TOKEN','NPM_TOKEN','NPM_STAGE_TOKEN','NPM_ID_TOKEN',
                  'ACTIONS_ID_TOKEN_REQUEST_TOKEN','ACTIONS_ID_TOKEN_REQUEST_URL'}


def clean_environment():
    env = {k:v for k,v in os.environ.items() if k not in CREDENTIAL_ENV and
           not k.lower().startswith('npm_config_') and not k.startswith('AJR_PUBLISH_ARTIFACT_')}
    env['GH_PROMPT_DISABLED'] = 'true'
    return env


def cli(argv, *, env=None, cwd=REPO, body=None, missing=False):
    result = subprocess.run(list(map(str,argv)), input=body, env=env or clean_environment(),
                            cwd=cwd, capture_output=True, timeout=180)
    if result.returncode:
        if missing and b'HTTP 404' in result.stderr:
            return None
        raise RuntimeError('Local platform command failed; private output withheld. Check the shared CLI login/session interactively, then retry the same candidate.')
    require(len(result.stdout) <= 128 * 1024 * 1024, 'Local platform response exceeds bound')
    return result.stdout


class GitHubCLI:
    """Use the official CLI's existing host/account credential store, never export its token."""
    before_write = None

    def raw(self, path, *, method='GET', payload=None, binary=False, missing=False, file=None):
        if method != 'GET' and self.before_write is not None:
            self.before_write()
        args = ['gh','api','--hostname','github.com','--method',method,
                '-H','X-GitHub-Api-Version: 2026-03-10',path]
        if binary:
            args += ['-H','Accept: application/octet-stream']
        body = None
        if payload is not None:
            args += ['--input','-']; body = json.dumps(payload).encode()
        if file is not None:
            args += ['--input',file,'-H','Content-Type: application/octet-stream']
        data = cli(args, body=body, missing=missing)
        return data if binary or data is None else json.loads(data)

    def request(self, path, **kwargs):
        return self.raw('repos/' + publisher.REPOSITORY + ('/' + path if path else ''), **kwargs)

    def listing(self, path):
        values = []
        for page in range(1,21):
            batch = self.request(f'{path}?per_page=100&page={page}')
            require(isinstance(batch,list), 'Unexpected GitHub inventory')
            values.extend(batch)
            if len(batch) < 100:
                return values
        raise ValueError('GitHub inventory exceeds publication bound')

    def check_tag(self, tag, revision):
        value = self.request('git/ref/tags/' + quote(tag,safe=''), missing=True)
        if value is None:
            return False
        obj = value['object']
        for _ in range(5):
            if obj.get('type') != 'tag':
                break
            obj = self.request('git/tags/' + obj['sha'])['object']
        require(obj.get('type') == 'commit' and obj.get('sha') == revision, 'Existing tag conflicts with the original source; never move it')
        return True

    def asset_bytes(self, asset):
        require(isinstance(asset.get('id'),int) and 0 < asset.get('size',0) <= 128 * 1024 * 1024, 'Invalid bounded GitHub asset')
        data = self.request(f"releases/assets/{asset['id']}",binary=True)
        require(len(data) == asset['size'] and (not asset.get('digest') or asset['digest'] == 'sha256:' + hashlib.sha256(data).hexdigest()), 'GitHub asset differs from declared bytes')
        return data

    def upload(self, release_id, path):
        require(isinstance(release_id,int) and release_id > 0, 'Invalid GitHub release identity')
        url = f"https://uploads.github.com/repos/{publisher.REPOSITORY}/releases/{release_id}/assets?name={quote(path.name,safe='')}"
        return self.raw(url,method='POST',file=path)

    def identity(self, expected):
        require(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9-]{0,38}',expected or ''), 'Explicit expected GitHub account is required')
        login = self.raw('user')['login']
        require(login.lower() == expected.lower() and self.request('')['full_name'] == publisher.REPOSITORY, 'Shared GitHub login belongs to another account/repository')
        return login


def local_context(operation, expected, *, root=REPO, env=None):
    env = os.environ if env is None else env
    require(not env.get('CI') and env.get('GITHUB_ACTIONS') != 'true' and not env.get('CI_PIPELINE_EVENT'),
            'Local maintainer entry point cannot run as a CI workflow')
    require(re.fullmatch(r'[0-9a-f]{40}',expected or '') and
            publisher.git(['rev-parse','HEAD'],root).strip() == expected and
            not publisher.git(['status','--porcelain','--untracked-files=no'],root).strip(), 'Explicit reviewed clean local revision is required')
    return {'provider':'local','repository':publisher.REPOSITORY,'workflow_commit':expected,
            'task':operation,'invocation':uuid.uuid4().hex,'created':int(time.time())}


def fetch(args, work):
    server = urlsplit(args.server); endpoint = urlsplit(args.exchange_endpoint)
    for url in (server,endpoint):
        require(url.scheme == 'https' and url.netloc and not url.username and not url.password and
                not url.query and not url.fragment and url.path in ('','/'), 'Expected credential-free HTTPS origin')
    require(re.fullmatch(r'[1-9][0-9]{0,9}',args.repo_id) and re.fullmatch(r'[1-9][0-9]{0,11}',args.pipeline), 'Invalid original repository/run identity')
    require(stat.S_IMODE(args.exchange_credentials.stat().st_mode) & 0o077 == 0,
            'Exchange credential file must be private to its owner')
    token = read_json(args.exchange_credentials).get(args.repo_id,{}).get('publisher')
    require(isinstance(token,str) and re.fullmatch(r'[A-Za-z0-9_-]{40,128}',token), 'Publisher read role is not configured for this project')
    request = work/'request.json';request.write_text(json.dumps({'repo_id':args.repo_id,'pipeline':args.pipeline,'server':args.server.rstrip('/')}))
    env = clean_environment();env.update(AJR_PUBLISH_ARTIFACT_ENDPOINT=args.exchange_endpoint.rstrip('/'),
        AJR_PUBLISH_ARTIFACT_CA=args.exchange_ca.read_text(),AJR_PUBLISH_ARTIFACT_TOKEN=token)
    cli(['node',REPO/'scripts/release_publish_artifacts.mjs',request,work/'candidate',work/'status.json'],env=env)
    status = read_json(work/'status.json')
    source = publisher.original_source(status['pipeline']['commit'],destination=work/'source')
    candidate = verify_prepared(work/'candidate',status['pipeline'],status['workflows'],source,server=args.server.rstrip('/'))
    return candidate, source


@contextmanager
def npm_environment(userconfig, work):
    require(userconfig.is_file() and stat.S_IMODE(userconfig.stat().st_mode) & 0o077 == 0,
            'Log in interactively with npm login --auth-type=web first; the account userconfig must be owner-private')
    env = clean_environment(); empty = work/'empty-global-npmrc';empty.touch(mode=0o600,exist_ok=True)
    env.update(npm_config_userconfig=str(userconfig.resolve()),npm_config_globalconfig=str(empty),
               npm_config_cache=str(work/'npm-cache'),npm_config_ignore_scripts='true')
    # The CLI reads the existing local session. This code never reads/copies its token.
    yield env


def npm_identity(expected, env, work):
    require(re.fullmatch(r'[a-zA-Z0-9_-]+',expected or ''), 'Explicit expected npm account is required')
    version = tuple(map(int,cli(['npm','--version'],env=env,cwd=work).decode().strip().split('.')[:3]))
    node = tuple(map(int,cli(['node','--version'],env=env,cwd=work).decode().strip().lstrip('v').split('.')[:3]))
    require(version >= (11,15,0) and node >= (22,14,0), 'Local staging needs npm >=11.15 and Node >=22.14')
    user = json.loads(cli(['npm','whoami','--json','--registry='+shared.REGISTRY],env=env,cwd=work))
    require(user == expected, 'Shared npm login belongs to another expected account')
    return user


def download_stage(stage_id, env, work):
    with tempfile.TemporaryDirectory(prefix='stage-download-',dir=work) as temp:
        directory = Path(temp)
        cli(['npm','stage','download',stage_id,'--registry='+shared.REGISTRY],env=env,cwd=directory)
        files = list(directory.iterdir())
        require(len(files) == 1 and files[0].suffix == '.tgz' and files[0].is_file() and not files[0].is_symlink() and
                files[0].stat().st_size <= 128 * 1024 * 1024, 'Unexpected staged download inventory')
        return files[0].read_bytes()


def operate(args, work, candidate, source, context, *, client=None):
    if args.operation == 'inspect':
        return {'operation':'inspect','candidate_pipeline':args.pipeline,'version':candidate['version'],
                'development_only':candidate['development'],'original_bytes_verified':True,'publishing_authorized':False,'writes':False}
    publisher.eligible(candidate,context)
    github = client or GitHubCLI(); context['github_user'] = github.identity(args.expected_github_user)
    require(github.request('commits/main')['sha'] == context['workflow_commit'], 'Local publication tool must match the reviewed current main revision')
    context['branch'] = 'main'
    def guard_write():
        require({p.name:sha256(p) for p in (work/'candidate').iterdir()} == publication.assets,
                'Original candidate bytes changed before a write; preserve them for inspection')
        require(publisher.git(['rev-parse','HEAD']).strip() == context['workflow_commit'] and
                not publisher.git(['status','--porcelain','--untracked-files=no']).strip() and
                github.request('commits/main')['sha'] == context['workflow_commit'],
                'Reviewed local/main source changed before a write; inspect the original candidate without rebuilding')
    if isinstance(github, GitHubCLI):
        github.before_write = guard_write
    publication = publisher.Publication(work/'candidate',candidate,source,context,client=github)
    if args.operation == 'finalize':
        return publication.finalize()
    if args.operation == 'check':
        return publication.check()
    # Check both package states and Release conflicts before the first staging write.
    existing = {k:shared.public_package(publication.meta[k]) for k in PACKAGES}
    for k in PACKAGES:
        shared.check_channel(publication.meta[k]['name'],publication.channel,publication.meta['version'])
    publication.find_release()
    with npm_environment(args.npm_userconfig,work) as env:
        user = npm_identity(args.expected_npm_user,env,work)
        verified = True
        def stage_cli(argv, env):
            if list(argv[:3]) == ['npm','stage','publish'] and '--dry-run' not in argv:
                guard_write()
            return cli(argv,env=env,cwd=work)
        for key in PACKAGES:
            if existing[key] is not None:
                continue
            entry = publication.meta[key]; tarball = work/'candidate'/entry['tarball']
            if args.operation == 'preview':
                cli(['npm','stage','publish',tarball,'--dry-run','--ignore-scripts','--access=public',
                     '--tag='+publication.channel,'--registry='+shared.REGISTRY],env=env,cwd=work)
            else:
                verified = shared.stage_package(entry,tarball,publication.channel,env,
                    download=lambda stage_id,scope:download_stage(stage_id,scope,work),report=lambda message:None,
                    run=stage_cli) and verified
    return {'operation':args.operation,'npm_user':user,'version':publication.meta['version'],
            'channel':publication.channel,'writes':args.operation == 'stage','github_writes':False,
            'status':'read_only_preview' if args.operation == 'preview' else 'awaiting_maintainer_2fa' if verified else 'awaiting_npm_validation',
            'instruction':'For staging, inspect exact stages and approve runtime then OpenCode with human 2FA; never approve automatically. Retry only the same original candidate.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation',choices=sorted(OPERATIONS))
    for name in ['server','repo-id','pipeline','expected-revision','exchange-endpoint']:
        parser.add_argument('--'+name,required=True)
    parser.add_argument('--exchange-ca',type=Path,required=True)
    parser.add_argument('--exchange-credentials',type=Path,required=True)
    parser.add_argument('--expected-github-user')
    parser.add_argument('--expected-npm-user')
    parser.add_argument('--npm-userconfig',type=Path,default=Path.home()/'.npmrc')
    parser.add_argument('--apply',action='store_true')
    args = parser.parse_args()
    require(args.apply == (args.operation in ('stage','finalize')), 'Only stage/finalize require --apply; read-only operations refuse it')
    context = local_context(args.operation,args.expected_revision)
    (REPO/'.tmp').mkdir(exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix='local-publish-',dir=REPO/'.tmp'))
    candidate, source = fetch(args,work)
    evidence = {'publisher':context,'original_candidate':candidate['execution'],
                'candidate_sha256':sha256(work/'candidate/candidate.json'),'state':'candidate_verified',
                'writes_requested':args.apply}
    (work/'intent.json').write_text(json.dumps(evidence,indent=2)+'\n')
    try:
        result = operate(args,work,candidate,source,context)
    except (ValueError,RuntimeError,OSError,subprocess.SubprocessError,KeyError,TypeError):
        (work/'operation.json').write_text(json.dumps({**evidence,'state':'stopped',
            'writes_may_have_occurred':args.apply,'recovery':'Inspect remote state and retry this same original candidate; never rebuild or replace it.'},indent=2)+'\n')
        raise
    (work/'operation.json').write_text(json.dumps({**evidence,'state':'completed','result':result},indent=2)+'\n')
    print(json.dumps({**result,'local_evidence':str(work/'operation.json')}))


if __name__ == '__main__':
    try:
        main()
    except (ValueError,RuntimeError,OSError,subprocess.SubprocessError,KeyError,TypeError) as error:
        print(str(error) if isinstance(error,(ValueError,RuntimeError)) else 'Local release stopped; private platform output withheld',file=sys.stderr)
        sys.exit(1)
