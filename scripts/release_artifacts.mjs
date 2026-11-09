// Separate manual-preparation identities, roles and storage; no CI promotion.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { chmodSync, existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { identity, packBundle, inspectBundle, extractBundle, transfer, MAX_BUNDLE } from './ci_artifacts.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const PRODUCERS = {
  'release-preflight': 'release-preflight', 'release-linux-native': 'release-linux',
  'release-windows-native': 'release-windows', 'release-packages': 'release-package',
  'release-windows-report': 'release-windows-packages', 'release-candidate': 'release-seal',
  'release-diagnostics': 'release-windows-packages',
};
const destinations = { 'release-preflight': 'preflight', 'release-linux-native': 'linux',
  'release-windows-native': 'windows', 'release-packages': 'packages', 'release-windows-report': 'windows-report' };
const consumers = {
  'release-preflight': ['release-linux', 'release-windows', 'release-package', 'release-windows-packages', 'release-seal'],
  'release-linux-native': ['release-package', 'release-seal'], 'release-windows-native': ['release-package', 'release-seal'],
  'release-packages': ['release-windows-packages', 'release-seal'], 'release-windows-report': ['release-seal'],
};
export const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function check(ok, message) { if (!ok) throw new Error(message); }

export function execution(env = process.env) {
  check(env.CI === 'woodpecker' && env.CI_PIPELINE_EVENT === 'manual', 'Manual Woodpecker preparation only');
  check(env.CI_REPO === 'MorningFrog/agent-job-relay' && ['prepare', 'rehearse'].includes(env.AJR_RELEASE_TASK), 'Unexpected repository or preparation task');
  check(['latest', 'next'].includes(env.AJR_RELEASE_CHANNEL), 'Explicit preparation channel required');
  const run = identity(env), branch = env.CI_COMMIT_BRANCH;
  check(run.commit === env.EXPECTED_RELEASE_REVISION, 'Expected full revision must match the actual pipeline');
  check(typeof branch === 'string' && branch && env.CI_COMMIT_REF === 'refs/heads/' + branch, 'Expected an actual branch checkout');
  check(env.AJR_RELEASE_TASK === 'rehearse' || branch === 'main', 'Formal preparation requires main');
  const url = new URL(env.CI_PIPELINE_URL);
  check(['http:', 'https:'].includes(url.protocol) && !url.username && !url.password && !url.search && !url.hash &&
    url.pathname === `/repos/${run.repo_id}/pipeline/${run.pipeline}`, 'Invalid actual pipeline URL');
  run.server = url.origin;
  return { format: 1, run, branch, task: env.AJR_RELEASE_TASK, channel: env.AJR_RELEASE_CHANNEL };
}

export function inspectRelease(bytes, context, kind) {
  const files = inspectBundle(bytes, context.run, kind, PRODUCERS);
  const names = Object.keys(files);
  const exact = expected => check(isDeepStrictEqual([...names].sort(), [...expected].sort()), 'Unexpected release file inventory');
  const json = name => JSON.parse(files[name]);
  if (kind === 'release-preflight') {
    exact(['preflight.json']);
    check(isDeepStrictEqual(json('preflight.json').execution, context), 'Preflight belongs to another channel/mode/branch/run');
  } else if (kind === 'release-linux-native' || kind === 'release-windows-native') {
    const linux = kind === 'release-linux-native', executable = linux ? 'agent-job-relay' : 'agent-job-relay.exe';
    const descriptor = linux ? 'linux-native.json' : 'windows-native.json'; exact([executable, descriptor]);
    const meta = json(descriptor);
    check(meta.format === 1 && meta.platform === (linux ? 'linux-x64' : 'win32-x64') &&
      meta.revision === context.run.commit && meta.sha256 === hash(files[executable]), 'Release native identity mismatch');
    if (linux) check(meta.compatibility?.glibc === '2.34' && meta.compatibility?.result === 'passed' && meta.compatibility?.sha256 === meta.sha256, 'Missing exact-byte compatibility check');
  } else if (kind === 'release-packages' || kind === 'release-candidate') {
    const meta = json('release.json');
    check((meta.development ?? false) === (context.task === 'rehearse') && isDeepStrictEqual(meta.platforms, ['linux-x64', 'win32-x64']), 'Release package mode/platform mismatch');
    const required = [meta.runtime.tarball, meta.opencode.tarball, meta.marketplace, 'release.json', 'SHA256SUMS'];
    if (kind === 'release-candidate') {
      required.push('candidate.json', 'release-notes.md', 'woodpecker-evidence.json');
      const candidate = json('candidate.json');
      check(candidate.format === 2 && candidate.provider === 'woodpecker' && isDeepStrictEqual(candidate.execution, context) &&
        candidate.development === (context.task === 'rehearse') && candidate.workflow === 'release-seal' && candidate.version === meta.version, 'Invalid preparation seal');
      check(isDeepStrictEqual(candidate.assets, Object.fromEntries(names.filter(n => !['candidate.json', 'SHA256SUMS'].includes(n)).map(n => [n, hash(files[n])]))), 'Sealed candidate assets differ');
    }
    exact(required);
    const checked = new Set();
    for (const line of files.SHA256SUMS.toString('utf8').trim().split(/\r?\n/)) {
      const m = /^([0-9a-f]{64})  ([a-zA-Z0-9][a-zA-Z0-9._-]*)$/.exec(line);
      check(m && m[2] !== 'SHA256SUMS' && !checked.has(m[2]) && Object.hasOwn(files, m[2]) && hash(files[m[2]]) === m[1], 'Release checksum mismatch');
      checked.add(m[2]);
    }
    check(checked.size === names.length - 1, 'Incomplete release checksum inventory');
  } else if (kind === 'release-windows-report') {
    exact(['report.json', 'evidence.json']); const evidence = json('evidence.json');
    check(isDeepStrictEqual(evidence.execution, context) && evidence.report_sha256 === hash(files['report.json']) &&
      /^[0-9a-f]{64}$/.test(evidence.packages_sha256) && json('report.json').result === 'passed', 'Invalid Windows package evidence');
  } else check(kind === 'release-diagnostics', 'Unsupported release object');
  return files;
}

function readRegular(path, maximum = MAX_BUNDLE) {
  const stat = lstatSync(path);
  check(stat.isFile() && !stat.isSymbolicLink() && stat.size <= maximum, 'Expected a bounded regular preparation file');
  return readFileSync(path);
}

function readFiles(directory) {
  const files = {}; let total = 0;
  for (const name of readdirSync(directory)) {
    const path = join(directory, name), stat = lstatSync(path);
    check(stat.isFile() && !stat.isSymbolicLink(), 'Release inputs must be regular files');
    total += stat.size; check(total <= MAX_BUNDLE && Object.keys(files).length < 256, 'Release input exceeds bounds');
    files[name] = readRegular(path);
  }
  return files;
}

async function main() {
  const [operation, kind] = process.argv.slice(2), context = execution();
  const git = args => execFileSync('git', ['-c', 'safe.directory=' + root, ...args], { cwd: root, encoding: 'utf8' }).trim();
  check(git(['rev-parse', 'HEAD']) === context.run.commit && !git(['status', '--porcelain', '--untracked-files=no']), 'Unchanged exact checkout required');
  if (operation === 'guard') return;
  const workflow = process.env.CI_WORKFLOW_NAME;
  if (operation === 'download') {
    check(consumers[kind]?.includes(workflow), 'Unexpected release consumer');
    const bytes = await transfer(context.run, kind, undefined, 'AJR_RELEASE');
    inspectRelease(bytes, context, kind);
    const parent = join(root, '.tmp/release-input'); mkdirSync(parent, { recursive: true });
    const destination = join(parent, destinations[kind]);
    extractBundle(bytes, context.run, kind, destination, PRODUCERS);
    if (kind === 'release-linux-native') chmodSync(join(destination, 'agent-job-relay'), 0o755);
    writeFileSync(join(parent, kind + '.sha256'), hash(bytes) + '\n', { flag: 'wx', mode: 0o600 });
  } else if (operation === 'upload') {
    check(Object.hasOwn(PRODUCERS, kind) && workflow === PRODUCERS[kind], 'Unexpected release producer');
    let files;
    if (kind === 'release-linux-native') files = { 'agent-job-relay': readRegular(join(root, 'target/release/agent-job-relay')), 'linux-native.json': readRegular(join(root, '.tmp/release-linux-native.json'), 1048576) };
    else if (kind === 'release-windows-native') files = { 'agent-job-relay.exe': readRegular(join(root, 'target/release/agent-job-relay.exe')), 'windows-native.json': readRegular(join(root, 'target/windows-native.json'), 1048576) };
    else if (kind === 'release-windows-report' || kind === 'release-diagnostics') {
      const reports = readdirSync(join(root, '.tmp')).filter(n => /^wp-[a-zA-Z0-9]+$/.test(n) && existsSync(join(root, '.tmp', n, 'report.json')));
      check(reports.length === 1, 'Expected one isolated Windows package report');
      const path = join(root, '.tmp', reports[0], 'report.json');
      check(lstatSync(path).isFile() && !lstatSync(path).isSymbolicLink() && lstatSync(path).size <= 1024 * 1024, 'Invalid bounded Windows report');
      const report = readFileSync(path);
      files = { 'report.json': report };
      if (kind === 'release-windows-report') files['evidence.json'] = Buffer.from(JSON.stringify({ format: 1, execution: context,
        packages_sha256: readFileSync(join(root, '.tmp/release-input/release-packages.sha256'), 'utf8').trim(), report_sha256: hash(report) }));
    } else files = readFiles(join(root, { 'release-preflight': '.tmp/release-preflight', 'release-packages': '.tmp/release-packages', 'release-candidate': '.tmp/release-candidate' }[kind]));
    const bytes = packBundle(context.run, kind, files, PRODUCERS); inspectRelease(bytes, context, kind);
    await transfer(context.run, kind, bytes, 'AJR_RELEASE');
  } else throw new Error('Usage: release_artifacts.mjs {guard|upload|download} [release-kind]');
  console.log('Verified manual ' + context.task + ' ' + operation + ' ' + kind + '; no publication');
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch(error => { console.error(error.message); process.exitCode = 1; });
