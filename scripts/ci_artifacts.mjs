// CI-only exchange: exact-run bytes, never release promotion or successful-test caching.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { request } from 'node:https';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const MAGIC = Buffer.from('AJRCI001');
export const MAX_BUNDLE = 256 * 1024 * 1024;
const PRODUCERS = { 'linux-native': 'linux', 'windows-native': 'windows', 'windows-packages': 'package-windows-test', 'windows-diagnostics': 'windows-packages' };
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function requireValue(value, message) { if (!value) throw new Error(message); }
function safeName(name) {
  return typeof name === 'string' && /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,179}$/.test(name) && !/[. ]$/.test(name) && !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name);
}

export function identity(env = process.env) {
  const value = { repository: env.CI_REPO, repo_id: env.CI_REPO_ID, pipeline: env.CI_PIPELINE_NUMBER, created: env.CI_PIPELINE_CREATED, commit: env.CI_COMMIT_SHA };
  requireValue(typeof value.repository === 'string' && /^[\w.-]+\/[\w.-]+$/.test(value.repository), 'Missing CI repository');
  for (const key of ['repo_id', 'pipeline', 'created']) requireValue(typeof value[key] === 'string' && /^[1-9][0-9]{0,11}$/.test(value[key]), 'Missing CI run identity: ' + key);
  requireValue(typeof value.commit === 'string' && /^[0-9a-f]{40}$/.test(value.commit), 'Missing CI commit identity');
  return value;
}

export function packBundle(run, kind, files, producers = PRODUCERS) {
  requireValue(Object.hasOwn(producers, kind), 'Unknown CI artifact kind');
  const entries = Object.entries(files).sort(([a], [b]) => a.localeCompare(b, 'en'));
  requireValue(entries.length > 0 && entries.length <= 256, 'Invalid CI file count');
  const names = new Set();
  for (const [name, data] of entries) {
    requireValue(safeName(name) && !names.has(name.toLowerCase()) && Buffer.isBuffer(data), 'Invalid CI filename or bytes');
    names.add(name.toLowerCase());
  }
  const manifest = { format: 1, identity: run, kind, producer: producers[kind], files: entries.map(([name, data]) => ({ name, size: data.length, sha256: hash(data) })) };
  const json = Buffer.from(JSON.stringify(manifest));
  requireValue(json.length <= 65536, 'CI manifest too large');
  const prefix = Buffer.alloc(12); MAGIC.copy(prefix); prefix.writeUInt32BE(json.length, 8);
  const total = 12 + json.length + entries.reduce((sum, [, data]) => sum + data.length, 0);
  requireValue(total <= MAX_BUNDLE, 'CI bundle too large');
  return Buffer.concat([prefix, json, ...entries.map(([, data]) => data)], total);
}

export function inspectBundle(bytes, run, kind, producers = PRODUCERS) {
  requireValue(bytes.length >= 12 && bytes.length <= MAX_BUNDLE && bytes.subarray(0, 8).equals(MAGIC), 'Invalid CI bundle');
  const length = bytes.readUInt32BE(8);
  requireValue(length > 0 && length <= 65536 && 12 + length <= bytes.length, 'Invalid CI manifest length');
  const manifest = JSON.parse(bytes.subarray(12, 12 + length).toString('utf8'));
  requireValue(manifest.format === 1 && isDeepStrictEqual(manifest.identity, run) && manifest.kind === kind && Object.hasOwn(producers, kind) && manifest.producer === producers[kind], 'CI artifact belongs to another source/run/producer');
  requireValue(Array.isArray(manifest.files) && manifest.files.length > 0 && manifest.files.length <= 256, 'Invalid CI file inventory');
  const names = new Set(), files = {};
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
  if (kind === 'windows-packages') {
    const meta = JSON.parse(files['release.json'] ?? 'null');
    requireValue(meta?.development === true && isDeepStrictEqual(meta.platforms, ['linux-x64', 'win32-x64']), 'Expected development-only dual-platform packages');
    const expected = [meta.runtime?.tarball, meta.opencode?.tarball, meta.marketplace, 'release.json', 'SHA256SUMS'];
    requireValue(expected.every(n => safeName(n) && Object.hasOwn(files, n)) && expected.length === names.size, 'Unexpected package inventory');
    const checksums = files.SHA256SUMS.toString('utf8').trim().split(/\r?\n/);
    requireValue(checksums.length === expected.length - 1, 'Invalid package checksum inventory');
    const checked = new Set();
    for (const line of checksums) {
      const match = /^([0-9a-f]{64})  ([a-zA-Z0-9][a-zA-Z0-9._-]*)$/.exec(line);
      requireValue(match && match[2] !== 'SHA256SUMS' && !checked.has(match[2]) && Object.hasOwn(files, match[2]) && hash(files[match[2]]) === match[1], 'Development package checksum mismatch');
      checked.add(match[2]);
    }
  }
  return files;
}

export function extractBundle(bytes, run, kind, destination, producers = PRODUCERS) {
  const files = inspectBundle(bytes, run, kind, producers); // Validate the entire object before writing.
  requireValue(!existsSync(destination), 'Choose a new CI artifact destination');
  const parent = realpathSync(dirname(destination));
  requireValue(resolve(parent, relative(dirname(destination), destination)) === resolve(destination), 'Refusing a symlinked artifact parent');
  mkdirSync(destination);
  try {
    for (const [name, data] of Object.entries(files)) writeFileSync(join(destination, name), data, { flag: 'wx', mode: kind === 'linux-native' ? 0o755 : 0o600 });
  } catch (error) { rmSync(destination, { recursive: true, force: true }); throw error; }
}

function config(prefix) {
  const endpoint = new URL(process.env[prefix + '_ARTIFACT_ENDPOINT']);
  requireValue(endpoint.protocol === 'https:' && !endpoint.username && !endpoint.password && !endpoint.search && !endpoint.hash && endpoint.pathname === '/', 'CI exchange requires a plain HTTPS origin');
  const token = process.env[prefix + '_ARTIFACT_TOKEN'], ca = process.env[prefix + '_ARTIFACT_CA'];
  requireValue(token && /^[a-zA-Z0-9_-]{40,128}$/.test(token) && ca?.includes('BEGIN CERTIFICATE'), 'Missing private CI exchange configuration');
  return { endpoint, token, ca };
}

export function transfer(run, kind, upload, prefix = 'AJR_CI') {
  return requestExchange('/v1/' + [run.repo_id, run.pipeline, run.created, run.commit, kind].join('/'), upload, prefix);
}
export function requestExchange(path, upload, prefix = 'AJR_CI') {
  const { endpoint, token, ca } = config(prefix);
  const target = new URL(path, endpoint);
  requireValue(target.origin === endpoint.origin, 'Refusing cross-origin exchange authentication');
  const headers = { Authorization: 'Bearer ' + token };
  if (upload) { headers['Content-Length'] = upload.length; headers['X-Content-SHA256'] = hash(upload); }
  return new Promise((accept, reject) => {
    const req = request(target, { method: upload ? 'PUT' : 'GET', ca, headers, timeout: 60000 }, response => {
      const chunks = []; let length = 0;
      response.on('data', chunk => { length += chunk.length; if (length > (upload ? 4096 : MAX_BUNDLE)) response.destroy(new Error('CI response exceeds bound')); else chunks.push(chunk); });
      response.on('error', reject);
      response.on('end', () => {
        try {
          requireValue((upload ? [200, 201] : [200]).includes(response.statusCode), 'CI exchange returned HTTP ' + response.statusCode);
          const bytes = Buffer.concat(chunks);
          if (!upload) requireValue(hash(bytes) === response.headers['x-content-sha256'], 'CI transfer checksum mismatch');
          accept(bytes);
        } catch (error) { reject(error); }
      });
    });
    req.on('error', reject); req.on('timeout', () => req.destroy(new Error('CI artifact transfer timed out')));
    req.end(upload);
  });
}

export async function uploadBundle(run, kind, files) {
  const bytes = packBundle(run, kind, files); inspectBundle(bytes, run, kind);
  await transfer(run, kind, bytes);
}
export async function downloadBundle(run, kind, destination) {
  extractBundle(await transfer(run, kind), run, kind, destination);
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

async function main() {
  const [operation, kind] = process.argv.slice(2);
  const run = identity();
  const git = args => execFileSync('git', ['-c', 'safe.directory=' + root, ...args], { cwd: root, encoding: 'utf8' }).trim();
  const head = git(['rev-parse', 'HEAD']);
  requireValue(head === run.commit, 'Checkout differs from CI artifact identity');
  requireValue(!git(['status', '--porcelain', '--untracked-files=no']), 'CI artifacts require unchanged tracked source');
  requireValue(Object.hasOwn(PRODUCERS, kind), 'Unknown CI artifact kind');
  if (operation === 'upload') {
    requireValue(process.env.CI_WORKFLOW_NAME === PRODUCERS[kind], 'Unexpected artifact producer workflow');
    let files;
    if (kind === 'linux-native') files = readFiles(join(root, 'target/release'), ['agent-job-relay']);
    else if (kind === 'windows-native') files = { ...readFiles(join(root, 'target/release'), ['agent-job-relay.exe']), ...readFiles(join(root, 'target'), ['windows-native.json']) };
    else if (kind === 'windows-packages') files = readFiles(join(root, '.tmp/ci-windows-packages'), readdirSync(join(root, '.tmp/ci-windows-packages')));
    else {
      files = {};
      let total = 0;
      for (const directory of readdirSync(join(root, '.tmp')).filter(n => /^wp-[a-zA-Z0-9]+$/.test(n))) {
        for (const name of readdirSync(join(root, '.tmp', directory)).filter(n => /(?:\.stdout|\.stderr|\.log|report\.json)$/.test(n))) {
          total += lstatSync(join(root, '.tmp', directory, name)).size;
          requireValue(total <= MAX_BUNDLE - 65548 && Object.keys(files).length < 256, 'CI diagnostics exceed bound; local evidence was preserved');
          Object.assign(files, Object.fromEntries(Object.entries(readFiles(join(root, '.tmp', directory), [name])).map(([n, bytes]) => [directory + '--' + n, bytes])));
        }
      }
      if (Object.keys(files).length === 0) files['diagnostics.json'] = Buffer.from(JSON.stringify({ message: 'No package diagnostics were produced', identity: run }));
    }
    await uploadBundle(run, kind, files);
    console.log('Stored immutable CI ' + kind + ' for pipeline ' + run.pipeline);
  } else if (operation === 'download') {
    requireValue(['package-windows-test', 'windows-packages'].includes(process.env.CI_WORKFLOW_NAME), 'Unexpected artifact consumer workflow');
    const destinations = { 'linux-native': '.tmp/ci-linux-native', 'windows-native': '.tmp/ci-windows-native', 'windows-packages': '.tmp/release-candidate' };
    requireValue(Object.hasOwn(destinations, kind), 'Unsupported CI download');
    mkdirSync(join(root, '.tmp'), { recursive: true });
    await downloadBundle(run, kind, join(root, destinations[kind]));
    console.log('Verified CI ' + kind + ' for pipeline ' + run.pipeline);
  } else throw new Error('Usage: node scripts/ci_artifacts.mjs {upload|download} <kind>');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch(error => { console.error(error.message); process.exitCode = 1; });
