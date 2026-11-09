#!/usr/bin/env python3
"""Manual Woodpecker preparation only; no publishing or Actions impersonation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit, quote

sys.dont_write_bytecode = True
from release_metadata import (REPO, PACKAGES, check_candidate, release_notes, require,
                              sha256, source_metadata, write_checksums)
from release_actions import request
from prepare_release import validate_windows_image

REPOSITORY = 'MorningFrog/agent-job-relay'
WORKFLOWS = {'release-preflight', 'release-linux', 'release-windows', 'release-package',
             'release-windows-packages', 'release-seal'}
LINUX_LIMIT = (2, 34)


def command(argv, root=REPO):
    return subprocess.check_output(list(map(str, argv)), cwd=root, text=True, timeout=60).strip()


def execution(env, root=REPO):
    require(env.get('CI') == 'woodpecker' and env.get('CI_PIPELINE_EVENT') == 'manual',
            'Release preparation requires an actual manual Woodpecker pipeline')
    require(env.get('CI_REPO') == REPOSITORY, 'Unexpected release repository')
    task = env.get('AJR_RELEASE_TASK')
    require(task in ('prepare', 'rehearse'), 'Select prepare or development-only rehearse explicitly')
    channel = env.get('AJR_RELEASE_CHANNEL')
    require(channel in ('latest', 'next'), 'Select a release channel explicitly')
    revision = env.get('CI_COMMIT_SHA', '')
    require(re.fullmatch(r'[0-9a-f]{40}', revision) is not None and
            env.get('EXPECTED_RELEASE_REVISION') == revision, 'Expected full source revision is required and must match the pipeline')
    branch = env.get('CI_COMMIT_BRANCH', '')
    require(env.get('CI_COMMIT_REF') == 'refs/heads/' + branch and branch,
            'Release preparation requires an explicit branch checkout, never a tag/PR ref')
    require(task == 'rehearse' or branch == 'main', 'Formal preparation must select main')
    run = {'repository': REPOSITORY, 'repo_id': env.get('CI_REPO_ID', ''),
           'pipeline': env.get('CI_PIPELINE_NUMBER', ''), 'created': env.get('CI_PIPELINE_CREATED', ''),
           'commit': revision}
    require(all(re.fullmatch(r'[1-9][0-9]{0,11}', run[k]) for k in ('repo_id', 'pipeline', 'created')),
            'Missing actual Woodpecker repository/run identity')
    url = urlsplit(env.get('CI_PIPELINE_URL', ''))
    require(url.scheme in ('http', 'https') and url.netloc and not url.username and not url.password
            and not url.query and not url.fragment and
            url.path == f"/repos/{run['repo_id']}/pipeline/{run['pipeline']}", 'Invalid actual pipeline URL')
    run['server'] = f'{url.scheme}://{url.netloc}'
    require(command(['git', '-c', 'safe.directory=' + str(root), 'rev-parse', 'HEAD'], root) == revision and
            not command(['git', '-c', 'safe.directory=' + str(root), 'status', '--porcelain', '--untracked-files=no'], root),
            'Candidate preparation requires the unchanged exact tracked checkout')
    return {'format': 1, 'run': run, 'branch': branch, 'task': task, 'channel': channel}


def unused_version(source, context, token):
    require(token and token != 'UNCONFIGURED', 'Formal source preflight requires a read-only GitHub contents token (including drafts)')
    api = f'https://api.github.com/repos/{REPOSITORY}'
    def get(path, **kwargs):
        return request(api + '/' + path, token=token, **kwargs)
    require(get('git/ref/heads/main')['object']['sha'] == context['run']['commit'],
            'main changed after preparation was requested; select its exact reviewed revision')
    require(get('git/ref/tags/' + quote(source['tag'], safe=''), missing=True) is None,
            'Version already has a tag; retain its original candidate')
    for page in range(1, 101):
        releases = get(f'releases?per_page=100&page={page}')
        require(not any(r['tag_name'] == source['tag'] for r in releases),
                'Version already has a release/draft; retain its original candidate')
        if len(releases) < 100:
            break
    else:
        raise RuntimeError('Release history exceeded the bounded source check')
    for name in PACKAGES.values():
        require(request('https://registry.npmjs.org/' + quote(name, safe='') + '/' + source['version'], missing=True) is None,
                'Version is partially/publicly published; never rebuild or relabel it')


def preflight(context, token=None, root=REPO):
    source = source_metadata(root, allow_unreleased=context['task'] == 'rehearse')
    if context['task'] == 'prepare':
        unused_version(source, context, token)
    return {'format': 1, 'execution': context, 'version': source['version'],
            'source_notes_sha256': hashlib.sha256(source['notes'].encode()).hexdigest(),
            'unused_version_checked': context['task'] == 'prepare'}


def read_json(path):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 1024 * 1024,
            'Expected a bounded regular preparation record')
    return json.loads(path.read_text())


def verify_preflight(record, context, root=REPO):
    source = source_metadata(root, allow_unreleased=context['task'] == 'rehearse')
    require(record == {'format': 1, 'execution': context, 'version': source['version'],
                       'source_notes_sha256': hashlib.sha256(source['notes'].encode()).hexdigest(),
                       'unused_version_checked': context['task'] == 'prepare'},
            'Source preflight belongs to another run/channel/mode/version/notes')
    return source


def linux_descriptor(binary, compatibility, context, version):
    image = binary.read_bytes()
    require(image[:6] == b'\x7fELF\x02\x01' and int.from_bytes(image[18:20], 'little') == 62,
            'Expected Linux x86_64 ELF')
    require(command([binary.resolve(), '--version']) == 'agent-job-relay ' + version, 'Native/source version mismatch')
    symbols = command(['readelf', '--version-info', binary.resolve()])
    versions = [tuple(map(int, v.split('.'))) for v in re.findall(r'\bGLIBC_(\d+\.\d+)\b', symbols)]
    require(versions and max(versions) <= LINUX_LIMIT, 'Linux payload exceeds the glibc 2.34 release boundary')
    digest = hashlib.sha256(image).hexdigest()
    require(compatibility == {'glibc': '2.34', 'sha256': digest, 'result': 'passed'},
            'Missing matching glibc 2.34 runtime smoke check')
    return {'format': 1, 'platform': 'linux-x64', 'version': version,
            'revision': context['run']['commit'], 'sha256': digest,
            'minimumGlibc': '.'.join(map(str, max(versions))), 'compatibility': compatibility}


def verify_native(linux, windows, context, version):
    descriptor = read_json(linux / 'linux-native.json')
    require(descriptor.get('platform') == 'linux-x64' and descriptor.get('version') == version and
            descriptor.get('revision') == context['run']['commit'] and
            descriptor.get('sha256') == sha256(linux / 'agent-job-relay'), 'Linux native evidence mismatch')
    require(re.fullmatch(r'\d+\.\d+', descriptor.get('minimumGlibc', '')) and
            tuple(map(int, descriptor['minimumGlibc'].split('.'))) <= LINUX_LIMIT,
            'Linux candidate exceeds the release compatibility boundary')
    require(descriptor.get('compatibility') == {'glibc': '2.34', 'sha256': descriptor['sha256'], 'result': 'passed'},
            'Missing exact-byte Linux compatibility evidence')
    win = read_json(windows / 'windows-native.json')
    validate_windows_image((windows / 'agent-job-relay.exe').read_bytes(), win, version, context['run']['commit'])
    return {'linux': descriptor, 'windows': win}


def verify_report(report, evidence, context, package_digest, version):
    require(evidence == {'format': 1, 'execution': context, 'packages_sha256': package_digest,
                         'report_sha256': hashlib.sha256(report).hexdigest()}, 'Installed-package report belongs to different bytes/run')
    doc = json.loads(report)
    require(doc.get('result') == 'passed' and doc.get('platform') == 'native Windows' and
            doc.get('npm_global_short_cli') is True and
            doc.get('installed_runtime', {}).get('version') == version and
            doc['installed_runtime'].get('history_preserved') is True and
            doc['installed_runtime'].get('corruption_refused') is True and
            doc['installed_runtime'].get('bundled_ui') is True and
            doc.get('codex', {}).get('actual_catalog') is True and doc['codex'].get('tools') == 27 and
            doc.get('opencode', {}).get('model') == 'simulated', 'Windows installed-package/loader evidence did not pass')


def seal(directory, destination, record, context, native, report, evidence, package_digest, root=REPO):
    verify_preflight(record, context, root)
    development = context['task'] == 'rehearse'
    meta = check_candidate(directory, root, allow_development=development)
    require(meta.get('development', False) is development and meta['platforms'] == ['linux-x64', 'win32-x64'],
            'Preparation mode/platforms differ from package bytes; development packages cannot become formal candidates')
    require(meta['minimumGlibc'] == native['linux']['minimumGlibc'], 'Packaged/native Linux compatibility differs')
    verify_report(report, evidence, context, package_digest, meta['version'])
    expected = {meta['runtime']['tarball'], meta['opencode']['tarball'], meta['marketplace'], 'release.json', 'SHA256SUMS'}
    require({p.name for p in directory.iterdir()} == expected, 'Unexpected/unsealed package inventory')
    require(not destination.exists() and not destination.is_symlink(), 'Choose a new candidate destination; never overwrite a seal')
    # Build the entire seal privately and publish its directory once; no half-seal retry.
    with tempfile.TemporaryDirectory(prefix='release-seal-', dir=destination.parent) as temp:
        stage = Path(temp) / 'candidate'
        shutil.copytree(directory, stage)
        (stage / 'release-notes.md').write_text(release_notes(meta, root, allow_development=development))
        verification = {'format': 1, 'preflight': record, 'native': native, 'windows': evidence,
                        'windows_report': json.loads(report), 'windows_report_raw': report.decode(), 'packages_sha256': package_digest}
        (stage / 'woodpecker-evidence.json').write_text(json.dumps(verification, indent=2) + '\n')
        candidate = {'format': 2, 'provider': 'woodpecker', 'execution': context,
                     'workflow': 'release-seal', 'version': meta['version'], 'development': development,
                     'assets': {p.name: sha256(p) for p in stage.iterdir() if p.name != 'SHA256SUMS'}}
        (stage / 'candidate.json').write_text(json.dumps(candidate, indent=2) + '\n')
        write_checksums(stage)
        os.rename(stage, destination)
    return candidate


def verify_prepared(directory, pipeline, workflows, root=REPO, *, server):
    candidate = read_json(directory / 'candidate.json')
    require(candidate.get('format') == 2 and candidate.get('provider') == 'woodpecker' and
            candidate.get('workflow') == 'release-seal', 'Expected a Woodpecker preparation seal, never an Actions candidate')
    context = candidate['execution']; run = context['run']
    require(context.get('format') == 1 and context.get('task') in ('prepare', 'rehearse') and
            context.get('channel') in ('latest', 'next') and run.get('repository') == REPOSITORY and
            run.get('server') == server and
            (context['task'] == 'rehearse' or context.get('branch') == 'main'), 'Invalid preparation policy')
    meta = check_candidate(directory, root, allow_development=context['task'] == 'rehearse')
    require(candidate['development'] is (context['task'] == 'rehearse') and
            meta.get('development', False) is candidate['development'] and candidate['version'] == meta['version'],
            'Candidate preparation mode/version mismatch')
    proof = read_json(directory / 'woodpecker-evidence.json')
    verify_preflight(proof['preflight'], context, root)
    verify_report(proof['windows_report_raw'].encode(), proof['windows'], context, proof['packages_sha256'], meta['version'])
    require(json.loads(proof['windows_report_raw']) == proof['windows_report'], 'Conflicting installed-package report evidence')
    require(tuple(map(int, meta['minimumGlibc'].split('.'))) <= LINUX_LIMIT and
            meta['minimumGlibc'] == proof['native']['linux']['minimumGlibc'], 'Candidate exceeds the release compatibility boundary')
    require(candidate['assets'] == {p.name: sha256(p) for p in directory.iterdir() if p.name not in ('candidate.json', 'SHA256SUMS')},
            'Candidate asset inventory differs from the seal')
    require(pipeline['status'] == 'success' and pipeline['event'] == 'manual' and
            str(pipeline['number']) == run['pipeline'] and str(pipeline['created']) == run['created'] and
            str(pipeline['repo_id']) == run['repo_id'] and pipeline['commit'] == run['commit'] and
            pipeline['branch'] == context['branch'], 'Candidate lacks exact successful manual Server execution')
    require({w['name'] for w in workflows} == WORKFLOWS and all(w['state'] == 'success' for w in workflows),
            'All six preparation workflows must succeed; a seal alone is not acceptance')
    return candidate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['preflight', 'guard', 'describe-linux', 'package', 'seal'])
    args = parser.parse_args()
    context = execution(os.environ)
    tmp = REPO / '.tmp'; tmp.mkdir(exist_ok=True)
    inputs = tmp / 'release-input'
    if args.operation == 'preflight':
        record = preflight(context, os.environ.get('GH_TOKEN'))
        dest = tmp / 'release-preflight'; dest.mkdir()
        (dest / 'preflight.json').write_text(json.dumps(record, indent=2) + '\n')
    else:
        record = read_json(inputs / 'preflight/preflight.json')
        source = verify_preflight(record, context)
        if args.operation == 'describe-linux':
            value = linux_descriptor(REPO / 'target/release/agent-job-relay', read_json(tmp / 'release-compatibility.json'), context, source['version'])
            (tmp / 'release-linux-native.json').write_text(json.dumps(value, indent=2) + '\n')
        elif args.operation in ('package', 'seal'):
            native = verify_native(inputs / 'linux', inputs / 'windows', context, source['version'])
            if args.operation == 'package':
                argv = [sys.executable, '-B', REPO / 'scripts/prepare_release.py', '--binary', inputs / 'linux/agent-job-relay',
                        '--windows-binary', inputs / 'windows/agent-job-relay.exe', '--windows-metadata', inputs / 'windows/windows-native.json',
                        '--expected-revision', context['run']['commit'], '--output', tmp / 'release-packages']
                if context['task'] == 'rehearse': argv.append('--allow-unreleased')
                subprocess.run(list(map(str, argv)), cwd=REPO, check=True)
            else:
                if context['task'] == 'prepare': unused_version(source, context, os.environ.get('GH_TOKEN'))
                seal(inputs / 'packages', tmp / 'release-candidate', record, context, native,
                     (inputs / 'windows-report/report.json').read_bytes(), read_json(inputs / 'windows-report/evidence.json'),
                     (inputs / 'release-packages.sha256').read_text().strip())
    print('Woodpecker ' + context['task'] + ' ' + args.operation + ' passed; no package publication performed.')


if __name__ == '__main__':
    try: main()
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr); sys.exit(1)
