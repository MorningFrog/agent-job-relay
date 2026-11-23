// Frame exact-run Actions artifacts and verify every byte before extraction.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const MAGIC = Buffer.from('AJRCI001');
const MAX_BUNDLE = 256 * 1024 * 1024;
const PRODUCERS = {
  'linux-native': 'linux',
  'windows-native': 'windows',
  'ci-packages': 'package',
  'release-packages': 'package',
  'release-candidate': 'candidate',
};

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function requireValue(value, message) { if (!value) throw new Error(message); }
function safeName(name) {
  return typeof name === 'string' && /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,179}$/.test(name) && !/[. ]$/.test(name) && !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name);
}

export function identity(env = process.env) {
  requireValue(env.GITHUB_ACTIONS === 'true', 'Artifacts require a GitHub Actions run');
  const value = {
    repository: env.GITHUB_REPOSITORY,
    repo_id: env.GITHUB_REPOSITORY_ID,
    run_id: env.GITHUB_RUN_ID,
    commit: env.GITHUB_SHA,
  };
  requireValue(typeof value.repository === 'string' && /^[\w.-]+\/[\w.-]+$/.test(value.repository), 'Missing CI repository');
  for (const key of ['repo_id', 'run_id'])
    requireValue(typeof value[key] === 'string' && /^[1-9][0-9]{0,19}$/.test(value[key]), 'Missing CI run identity: ' + key);
  requireValue(typeof value.commit === 'string' && /^[0-9a-f]{40}$/.test(value.commit), 'Missing CI commit identity');
  return value;
}

export function packBundle(run, kind, files) {
  requireValue(Object.hasOwn(PRODUCERS, kind), 'Unknown CI artifact kind');
  const entries = Object.entries(files).sort(([a], [b]) => a.localeCompare(b, 'en'));
  requireValue(entries.length > 0 && entries.length <= 256, 'Invalid CI file count');
  const names = new Set();
  for (const [name, data] of entries) {
    requireValue(safeName(name) && !names.has(name.toLowerCase()) && Buffer.isBuffer(data), 'Invalid CI filename or bytes');
    names.add(name.toLowerCase());
  }
  const manifest = { format: 1, identity: run, kind, producer: PRODUCERS[kind], files: entries.map(([name, data]) => ({ name, size: data.length, sha256: hash(data) })) };
  const json = Buffer.from(JSON.stringify(manifest));
  requireValue(json.length <= 65536, 'CI manifest too large');
  const prefix = Buffer.alloc(12); MAGIC.copy(prefix); prefix.writeUInt32BE(json.length, 8);
  const total = 12 + json.length + entries.reduce((sum, [, data]) => sum + data.length, 0);
  requireValue(total <= MAX_BUNDLE, 'CI bundle too large');
  return Buffer.concat([prefix, json, ...entries.map(([, data]) => data)], total);
}

export function inspectBundle(bytes, run, kind) {
  requireValue(bytes.length >= 12 && bytes.length <= MAX_BUNDLE && bytes.subarray(0, 8).equals(MAGIC), 'Invalid CI bundle');
  const length = bytes.readUInt32BE(8);
  requireValue(length > 0 && length <= 65536 && 12 + length <= bytes.length, 'Invalid CI manifest length');
  const manifest = JSON.parse(bytes.subarray(12, 12 + length).toString('utf8'));
  requireValue(manifest.format === 1 && isDeepStrictEqual(manifest.identity, run) && manifest.kind === kind && Object.hasOwn(PRODUCERS, kind) && manifest.producer === PRODUCERS[kind], 'CI artifact belongs to another source/run/producer');
  requireValue(Array.isArray(manifest.files) && manifest.files.length > 0 && manifest.files.length <= 256, 'Invalid CI file inventory');
  const names = new Set(), files = Object.create(null);
  let cursor = 12 + length;
  for (const entry of manifest.files) {
    requireValue(entry && safeName(entry.name) && !names.has(entry.name.toLowerCase()) && Number.isSafeInteger(entry.size) && entry.size >= 0 && /^[0-9a-f]{64}$/.test(entry.sha256), 'Unsafe CI file inventory');
    requireValue(cursor + entry.size <= bytes.length, 'Truncated CI file');
    const data = bytes.subarray(cursor, cursor + entry.size);
    requireValue(hash(data) === entry.sha256, 'CI file checksum mismatch');
    names.add(entry.name.toLowerCase()); files[entry.name] = data; cursor += entry.size;
  }
  requireValue(cursor === bytes.length, 'Unexpected trailing CI data');
  if (kind === 'linux-native') requireValue(names.size === 1 && names.has('agent-job-relay'), 'Invalid Linux artifact');
  if (kind === 'windows-native') {
    requireValue(names.size === 2 && names.has('agent-job-relay.exe') && names.has('windows-native.json'), 'Invalid Windows artifact');
    const meta = JSON.parse(files['windows-native.json']);
    requireValue(meta.format === 1 && meta.platform === 'win32-x64' && meta.revision === run.commit && meta.sha256 === hash(files['agent-job-relay.exe']), 'Windows native identity mismatch');
  }
  if (['ci-packages', 'release-packages', 'release-candidate'].includes(kind)) {
    const meta = JSON.parse(files['release.json'] ?? 'null');
    const modeMatches = kind === 'ci-packages' ? meta?.development === true : meta?.development === undefined;
    requireValue(modeMatches && isDeepStrictEqual(meta?.platforms, ['linux-x64', 'win32-x64']), 'Unexpected package mode or platforms');
    const expected = [meta.runtime?.tarball, meta.opencode?.tarball, meta.marketplace, 'release.json', 'SHA256SUMS'];
    if (kind === 'release-candidate') {
      expected.push('candidate.json', 'release-notes.md');
      const seal = JSON.parse(files['candidate.json'] ?? 'null');
      requireValue(seal?.format === 1 && seal.repository === run.repository && seal.run_id === run.run_id &&
        seal.revision === run.commit && seal.workflow === '.github/workflows/release.yml', 'Candidate seal belongs to another run');
    }
    requireValue(expected.every(n => safeName(n) && Object.hasOwn(files, n)) &&
      new Set(expected).size === expected.length && expected.length === names.size, 'Unexpected package inventory');
    const checksums = files.SHA256SUMS.toString('utf8').trim().split(/\r?\n/);
    requireValue(checksums.length === expected.length - 1, 'Invalid package checksum inventory');
    const checked = new Set();
    for (const line of checksums) {
      const match = /^([0-9a-f]{64})  ([a-zA-Z0-9][a-zA-Z0-9._-]*)$/.exec(line);
      requireValue(match && match[2] !== 'SHA256SUMS' && !checked.has(match[2]) && Object.hasOwn(files, match[2]) && hash(files[match[2]]) === match[1], 'Development package checksum mismatch');
      checked.add(match[2]);
    }
  }
  return { ...files };
}

export function extractBundle(bytes, run, kind, destination) {
  const files = inspectBundle(bytes, run, kind); // Validate the entire object before writing.
  requireValue(!existsSync(destination), 'Choose a new CI artifact destination');
  const parent = realpathSync(dirname(destination));
  requireValue(resolve(parent, relative(dirname(destination), destination)) === resolve(destination), 'Refusing a symlinked artifact parent');
  mkdirSync(destination);
  try {
    for (const [name, data] of Object.entries(files)) writeFileSync(join(destination, name), data, { flag: 'wx', mode: kind === 'linux-native' ? 0o755 : 0o600 });
  } catch (error) { rmSync(destination, { recursive: true, force: true }); throw error; }
}

function readFiles(directory, names) {
  const files = {};
  let total = 0;
  requireValue(names.length > 0 && names.length <= 256, 'CI file count exceeds bound');
  for (const name of names) {
    requireValue(safeName(name) && lstatSync(join(directory, name)).isFile() && !lstatSync(join(directory, name)).isSymbolicLink(), 'Expected a regular CI file');
    total += lstatSync(join(directory, name)).size;
    requireValue(total <= MAX_BUNDLE, 'CI input files exceed bound');
    files[name] = readFileSync(join(directory, name));
  }
  return files;
}

function main() {
  const [operation, kind] = process.argv.slice(2);
  const current = identity();
  let run = current;
  if (operation === 'extract' && kind === 'release-candidate') {
    requireValue(process.env.CANDIDATE_RUN_ID && process.env.SEALED_REVISION, 'Missing original candidate identity');
    run = identity({ ...process.env, GITHUB_RUN_ID: process.env.CANDIDATE_RUN_ID, GITHUB_SHA: process.env.SEALED_REVISION });
  }
  // The container user can differ from the owner of the runner's mounted checkout.
  const git = args => execFileSync('git', ['-c', `safe.directory=${root}`, ...args], {
    cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
  requireValue(git(['rev-parse', 'HEAD']) === current.commit, 'Checkout differs from CI artifact identity');
  requireValue(!git(['status', '--porcelain', '--untracked-files=no']), 'CI artifacts require unchanged tracked source');
  requireValue(Object.hasOwn(PRODUCERS, kind), 'Unknown CI artifact kind');
  const directory = join(root, '.tmp');
  const bundle = join(directory, kind + '.ajr');
  const destination = join(directory, kind);
  mkdirSync(directory, { recursive: true });
  if (operation === 'pack') {
    let files;
    if (kind === 'linux-native') files = readFiles(join(root, 'target/release'), ['agent-job-relay']);
    else if (kind === 'windows-native') files = {
      ...readFiles(join(root, 'target/release'), ['agent-job-relay.exe']),
      ...readFiles(join(root, 'target'), ['windows-native.json']),
    };
    else files = readFiles(destination, readdirSync(destination));
    const bytes = packBundle(run, kind, files);
    inspectBundle(bytes, run, kind);
    writeFileSync(bundle, bytes, { flag: 'wx' });
  } else if (operation === 'extract') {
    const info = lstatSync(bundle);
    requireValue(info.isFile() && info.size <= MAX_BUNDLE, 'Expected a bounded regular CI bundle');
    extractBundle(readFileSync(bundle), run, kind, destination);
  } else throw new Error('Usage: node scripts/ci_artifacts.mjs {pack|extract} <kind>');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
