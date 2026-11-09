#!/usr/bin/env python3
"""Manual Woodpecker inspection, staged publication and read-only public checks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from urllib.parse import quote, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener

sys.dont_write_bytecode = True
import release_actions as shared
import release_metadata as metadata
from release_metadata import REPO, PACKAGES, require, sha256, source_metadata
from release_woodpecker import REPOSITORY, WORKFLOWS, verify_prepared, read_json

OPERATIONS = {'inspect', 'identity', 'preview', 'stage', 'finalize', 'check'}
RECOVERY_PATHS = shared.RECOVERY_PATHS | {
    'scripts/release_publish.py', 'scripts/release_publish_artifacts.mjs', 'scripts/release_artifacts.mjs',
    'scripts/ci_artifacts.mjs', 'scripts/release_metadata.py', 'scripts/test_release_publish.py',
    '.woodpecker/publish-inspect.yaml', '.woodpecker/publish-identity.yaml', '.woodpecker/publish-preview.yaml',
    '.woodpecker/publish-stage.yaml', '.woodpecker/publish-finalize.yaml', '.woodpecker/publish-check.yaml',
    'scripts/README.md', 'docs/ci.md',
}
SOURCE_PATHS = {'Cargo.toml', 'Cargo.lock', 'CHANGELOG.md', *metadata.PLUGIN_MANIFESTS,
                *metadata.RUNTIME_LAUNCHERS, *[p + '/package.json' for p in metadata.PACKAGE_PATHS.values()]}
RECEIPT_NAME = re.compile(r'woodpecker-publish-receipt-([1-9][0-9]*)-([1-9][0-9]*)\.json')


def git(args, root=REPO, binary=False):
    return shared.command(['git', '-c', 'safe.directory=' + str(root), *args], cwd=root, binary=binary)


def context(env, root=REPO):
    mode = env.get('AJR_RELEASE_TASK')
    require(env.get('CI') == 'woodpecker' and env.get('CI_PIPELINE_EVENT') == 'manual' and
            mode in OPERATIONS and env.get('CI_REPO') == REPOSITORY, 'Manual Woodpecker publication operation required')
    branch = env.get('CI_COMMIT_BRANCH', '')
    require(branch and env.get('CI_COMMIT_REF') == 'refs/heads/' + branch and
            (mode == 'inspect' or branch == 'main'), 'Publication operations must select main; only read-only inspection allows a reviewed branch')
    revision = env.get('CI_COMMIT_SHA', '')
    require(re.fullmatch(r'[0-9a-f]{40}', revision) and env.get('EXPECTED_RELEASE_REVISION') == revision,
            'Explicit expected workflow revision must match the actual checkout')
    require(git(['rev-parse', 'HEAD'], root).strip() == revision and
            not git(['status', '--porcelain', '--untracked-files=no'], root).strip(), 'Unchanged exact workflow source is required')
    ids = {k: env.get(v, '') for k,v in [('repo_id','CI_REPO_ID'),('pipeline','CI_PIPELINE_NUMBER'),('created','CI_PIPELINE_CREATED')]}
    require(all(re.fullmatch(r'[1-9][0-9]{0,11}', x) for x in ids.values()), 'Missing actual publication run identity')
    url = urlsplit(env.get('CI_PIPELINE_URL', ''))
    require(url.scheme in ('https','http') and url.netloc and not url.username and not url.password and
            not url.query and not url.fragment and url.path == f"/repos/{ids['repo_id']}/pipeline/{ids['pipeline']}", 'Invalid publication run origin')
    return {'provider':'woodpecker', 'run':{'repository':REPOSITORY, **ids, 'commit':revision,
            'server':f'{url.scheme}://{url.netloc}'}, 'branch':branch, 'task':mode}


def original_source(revision, root=REPO):
    require(re.fullmatch(r'[0-9a-f]{40}', revision), 'Invalid candidate source')
    git(['cat-file','-e', revision + '^{commit}'], root)
    directory = root / '.tmp' / ('publication-source-' + revision)
    require(not directory.exists(), 'Use a new publication workspace; do not overwrite candidate source')
    directory.mkdir(mode=0o700)
    # Read only the original metadata blobs. Never execute old source or unpack paths.
    for name in sorted(SOURCE_PATHS):
        path = directory / name; path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(git(['show', revision + ':' + name], root, binary=True))
    return directory


def eligible(candidate, ctx, root=REPO):
    require(not candidate['development'] and candidate['execution']['task'] == 'prepare' and
            candidate['execution']['branch'] == 'main', 'Development/rehearsal candidates cannot enter publication')
    original, current = candidate['execution']['run']['commit'], ctx['run']['commit']
    if original != current:
        git(['merge-base','--is-ancestor', original, current], root)
        changed = set(git(['diff','--name-only','-z', original, current, '--'], root, binary=True).decode().split('\0')) - {''}
        require(changed and changed <= RECOVERY_PATHS,
                'Publication tooling differs beyond the recovery allowlist; select the original reviewed source, never rebuild a partially published version')


def identity(expected_user):
    require(re.fullmatch(r'[a-zA-Z0-9_-]+', expected_user or ''), 'Explicit expected npm account is required')
    require(os.environ.get('NPM_STAGE_TOKEN') not in (None, '', 'UNCONFIGURED') and
            os.environ.get('GH_TOKEN') not in (None, '', 'UNCONFIGURED'), 'Configure the restricted npm stage token and repository GitHub credential privately')
    with shared.npm_environment('stage') as env:
        # Credentials are needed only by npm itself, never tarball lifecycle hooks.
        env.pop('GH_TOKEN', None); env.pop('GITHUB_TOKEN', None)
        user = json.loads(shared.command(['npm','whoami','--json',f'--registry={shared.REGISTRY}'], env=env))
        require(user == expected_user, 'npm token belongs to another expected account')
    require(shared.github('')['full_name'] == REPOSITORY, 'Unexpected GitHub repository identity')
    return {'npm_user':user, 'github_repository_read':True, 'stage_write_proven':False, 'github_write_proven':False}


def public_bytes(directory, meta, channel):
    for key in PACKAGES:
        entry = meta[key]; published = shared.public_package(entry)
        require(published is not None, 'Both npm packages must be public after maintainer 2FA approval; resume the same original candidate')
        require(shared.check_channel(entry['name'], channel, entry['version']).get(channel) == entry['version'], 'Public channel has not selected the sealed version; do not move tags automatically')
        url = published.get('dist',{}).get('tarball','')
        require(url.startswith(shared.REGISTRY + '/'), 'Unexpected public npm tarball origin')
        require(hashlib.sha256(shared.request(url, binary=True)).hexdigest() == sha256(directory / entry['tarball']), 'Public npm bytes differ from the original candidate')


class AssetRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        url = urlsplit(newurl)
        require(url.scheme == 'https' and not url.username and not url.password, 'Refusing unsafe GitHub asset redirect')
        # GitHub returns signed download redirects. Never forward its bearer token.
        return Request(newurl, headers={'Accept':'application/octet-stream','User-Agent':'AgentJobRelay-release'})


def asset_bytes(asset):
    require(isinstance(asset.get('id'), int) and 0 < asset.get('size', 0) <= 128 * 1024 * 1024, 'Invalid bounded GitHub asset')
    url = f"https://api.github.com/repos/{REPOSITORY}/releases/assets/{asset['id']}"
    req = Request(url, headers={'Authorization':'Bearer ' + os.environ.get('GH_TOKEN',''),
                              'Accept':'application/octet-stream','User-Agent':'AgentJobRelay-release'})
    try:
        with build_opener(AssetRedirect).open(req, timeout=30) as response:
            data = response.read(128 * 1024 * 1024 + 1)
    except Exception:
        raise RuntimeError('GitHub asset download failed; signed URLs and credentials were withheld') from None
    require(len(data) == asset['size'] and (not asset.get('digest') or asset['digest'] == 'sha256:' + hashlib.sha256(data).hexdigest()), 'GitHub asset size/digest mismatch')
    return data


class Publication:
    def __init__(self, directory, candidate, source, ctx):
        require(candidate.get('format') == 2 and candidate.get('provider') == 'woodpecker', 'Expected an unchanged Woodpecker candidate, never a relabeled Actions candidate')
        self.directory, self.candidate, self.source, self.context = directory, candidate, source, ctx
        self.meta = metadata.check_candidate(directory, source)
        self.execution = candidate['execution']; self.run = self.execution['run']; self.channel = self.execution['channel']
        self.assets = {p.name: sha256(p) for p in directory.iterdir()}
        self.marker = '<!-- AgentJobRelay Woodpecker candidate ' + json.dumps(self.execution, sort_keys=True, separators=(',',':')) + ' -->'
        self.body = (directory / 'release-notes.md').read_text() + '\n' + self.marker

    def receipt_base(self):
        return {'format':2,'kind':'npm_public_bytes_confirmed','candidate':self.execution,'version':self.meta['version'],
                'channel':self.channel,'packages':{entry['name']:{'integrity':entry['integrity'], 'sha256':sha256(self.directory/entry['tarball'])} for entry in [self.meta[k] for k in PACKAGES]}}

    def verify_receipt(self, asset):
        match = RECEIPT_NAME.fullmatch(asset['name']); require(match, 'Foreign GitHub release asset; nothing will be overwritten')
        receipt = json.loads(asset_bytes(asset)); publisher = receipt.pop('publisher', {})
        require(receipt == self.receipt_base() and publisher.get('provider') == 'woodpecker' and publisher.get('branch') == 'main' and publisher.get('task') == 'finalize', 'Publication receipt does not match the original candidate')
        run = publisher.get('run',{})
        require(run.get('repository') == REPOSITORY and run.get('server') == self.run['server'] and run.get('repo_id') == self.run['repo_id'] and
                run.get('pipeline') == match[1] and run.get('created') == match[2] and re.fullmatch(r'[0-9a-f]{40}',run.get('commit','')), 'Invalid publication run receipt')

    def find_release(self):
        shared.check_tag('v' + self.meta['version'], self.run['commit'])
        releases = [r for r in shared.github_list('releases') if r['tag_name'] == 'v' + self.meta['version']]
        require(len(releases) <= 1, 'Multiple Releases/drafts claim this version; preserve them for inspection')
        if not releases: return None
        release = releases[0]
        require(release['target_commitish'] == self.run['commit'] and release['body'] == self.body and
                release['prerelease'] == (self.channel == 'next'), 'Existing Release belongs to different/modified candidate bytes; nothing will be overwritten')
        assets = shared.github_list(f"releases/{release['id']}/assets")
        require(len(assets) == len({a['name'] for a in assets}), 'Duplicate GitHub assets')
        for asset in assets:
            if asset['name'] not in self.assets:
                self.verify_receipt(asset); continue
            expected = self.assets[asset['name']]
            if asset.get('digest'):
                require(asset['digest'] == 'sha256:' + expected, 'Existing GitHub bytes conflict with the original candidate')
            else:
                require(hashlib.sha256(asset_bytes(asset)).hexdigest() == expected, 'Existing GitHub bytes conflict with the original candidate')
        require(release['draft'] or set(self.assets) <= {a['name'] for a in assets}, 'Published Release lacks original assets; preserve it and inspect')
        return {**release,'_assets':assets}

    def preview(self, expected_user):
        eligible(self.candidate, self.context); auth = identity(expected_user)
        # Complete both public/Release conflict checks before even a local dry-run.
        existing = {k:shared.public_package(self.meta[k]) for k in PACKAGES}
        for k in PACKAGES: shared.check_channel(self.meta[k]['name'], self.channel, self.meta['version'])
        self.find_release()
        with shared.npm_environment('stage') as env:
            env.pop('GH_TOKEN',None); env.pop('GITHUB_TOKEN',None)
            for key in PACKAGES:
                if existing[key] is None:
                    shared.command(['npm','stage','publish',self.directory/self.meta[key]['tarball'],'--dry-run','--ignore-scripts','--access=public',f'--tag={self.channel}',f'--registry={shared.REGISTRY}'], env=env)
        return {'operation':'preview',**auth,'registry_or_github_writes':False}

    def stage(self, expected_user):
        eligible(self.candidate, self.context); identity(expected_user)
        existing = {k:shared.public_package(self.meta[k]) for k in PACKAGES}
        for key in PACKAGES: shared.check_channel(self.meta[key]['name'], self.channel, self.meta['version'])
        self.find_release()  # Read only. Do not create a GitHub draft before npm approval.
        stages_verified = True
        with shared.npm_environment('stage') as env:
            env.pop('GH_TOKEN',None); env.pop('GITHUB_TOKEN',None)
            for key in PACKAGES:
                if existing[key] is None:
                    stages_verified = shared.stage_package(self.meta[key],self.directory/self.meta[key]['tarball'],self.channel,env) and stages_verified
        return {'operation':'stage','status':'awaiting_maintainer_2fa' if stages_verified else 'awaiting_npm_validation',
                'version':self.meta['version'],'channel':self.channel,'github_writes':False,
                'instruction':'Review original staged bytes, approve runtime then OpenCode with npm 2FA, then manually finalize this same candidate. No approval is automated.'}

    def upload_asset(self, release_id, path):
        require(isinstance(release_id, int) and release_id > 0, 'Invalid GitHub Release ID')
        data = path.read_bytes()
        asset = shared.request(f"https://uploads.github.com/repos/{REPOSITORY}/releases/{release_id}/assets?name={quote(path.name, safe='')}",
            token=os.environ.get('GH_TOKEN'), method='POST', payload=data, context='GitHub release asset upload')
        require(asset.get('name') == path.name and asset.get('size') == len(data), 'Unexpected asset acknowledgement; inspect before retrying')
        expected = hashlib.sha256(data).hexdigest()
        require(asset.get('digest') == 'sha256:' + expected if asset.get('digest') else hashlib.sha256(asset_bytes(asset)).hexdigest() == expected,
                'Uploaded GitHub asset differs from the original candidate')

    def finalize(self):
        eligible(self.candidate,self.context)
        public_bytes(self.directory,self.meta,self.channel)  # Before the first GitHub mutation.
        release = self.find_release()
        if release is None:
            release = shared.github('releases',method='POST',payload={'tag_name':'v'+self.meta['version'],
                'target_commitish':self.run['commit'],'name':'AgentJobRelay '+self.meta['version'],
                'body':self.body,'draft':True,'prerelease':self.channel=='next'})
            require(release['body']==self.body and release['target_commitish']==self.run['commit'] and release['draft'], 'Unexpected draft acknowledgement; inspect before retrying')
            release['_assets']=[]
        existing = {a['name'] for a in release['_assets']}
        require(release['draft'] or set(self.assets) <= existing, 'Published Release is incomplete; do not replace assets')
        for name in sorted(set(self.assets)-existing):
            self.upload_asset(release['id'],self.directory/name)
        if release['draft']:
            receipt = {**self.receipt_base(),'publisher':self.context}
            name = f"woodpecker-publish-receipt-{self.context['run']['pipeline']}-{self.context['run']['created']}.json"
            with tempfile.TemporaryDirectory(prefix='publish-receipt-',dir=REPO/'.tmp') as temp:
                path=Path(temp)/name;path.write_text(json.dumps(receipt,sort_keys=True,indent=2)+'\n')
                self.upload_asset(release['id'],path)
            shared.github(f"releases/{release['id']}",method='PATCH',payload={'draft':False,'make_latest':'true' if self.channel=='latest' else 'false'})
        return {'operation':'finalize','status':'github_release_submitted','version':self.meta['version'],
                'registry_writes':False,'instruction':'Run the independent read-only check; never rebuild or republish to recover a failed check.'}

    def check(self):
        public_bytes(self.directory,self.meta,self.channel)
        release=self.find_release();require(release is not None and not release['draft'], 'GitHub Release is not public; resume finalization of the original candidate')
        require(shared.check_tag('v'+self.meta['version'],self.run['commit']), 'Public release tag is missing')
        for asset in release['_assets']:
            if asset['name'] in self.assets:
                require(hashlib.sha256(asset_bytes(asset)).hexdigest()==self.assets[asset['name']], 'Public GitHub bytes differ from the sealed candidate')
        shared.verify_public(self.directory,source_root=self.source)
        return {'operation':'check','status':'public_bytes_channels_installation_and_github_verified','writes':False}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation',choices=['request',*sorted(OPERATIONS)])
    parser.add_argument('--apply',action='store_true')
    args=parser.parse_args();ctx=context(os.environ);mode=ctx['task']
    require(args.operation in ('request',mode), 'Operation differs from the actual manual workflow task')
    require(args.apply == (mode in ('stage','finalize') and args.operation!='request'), 'Writes require the explicit apply invocation; inspect/preview/check never accept it')
    (REPO/'.tmp').mkdir(exist_ok=True)
    if args.operation=='identity':
        print(json.dumps({'operation':'identity',**identity(os.environ.get('NPM_EXPECTED_USER')),'writes':False}));return
    number=os.environ.get('AJR_CANDIDATE_PIPELINE_NUMBER','')
    require(re.fullmatch(r'[1-9][0-9]{0,11}',number), 'Explicit original candidate pipeline number is required')
    if args.operation=='request':
        path=REPO/'.tmp/publication-request.json'
        with path.open('x') as stream:json.dump({'repo_id':ctx['run']['repo_id'],'pipeline':number,'server':ctx['run']['server']},stream)
        return
    status=read_json(REPO/'.tmp/publication-status.json');directory=REPO/'.tmp/publication-candidate'
    require(status['server']==ctx['run']['server'] and str(status['pipeline']['repo_id'])==ctx['run']['repo_id'] and str(status['pipeline']['number'])==number, 'Status source differs from requested candidate')
    source=original_source(status['pipeline']['commit'])
    candidate=verify_prepared(directory,status['pipeline'],status['workflows'],source,server=ctx['run']['server'])
    if mode=='inspect':
        print(json.dumps({'operation':'inspect','candidate_pipeline':number,'version':candidate['version'],
            'development_only':candidate['development'],'formal_candidate':not candidate['development'] and candidate['execution']['task']=='prepare' and candidate['execution']['branch']=='main',
            'publishing_authorized':False,
            'original_bytes_verified':True,'writes':False}));return
    eligible(candidate,ctx)
    publication=Publication(directory,candidate,source,ctx)
    result = publication.preview(os.environ.get('NPM_EXPECTED_USER')) if mode=='preview' else publication.stage(os.environ.get('NPM_EXPECTED_USER')) if mode=='stage' else publication.finalize() if mode=='finalize' else publication.check()
    print(json.dumps(result))


if __name__=='__main__':
    try: main()
    except (ValueError,RuntimeError,OSError,subprocess.SubprocessError) as error:
        print(str(error),file=sys.stderr);sys.exit(1)
