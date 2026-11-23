import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { identity, packBundle, inspectBundle, extractBundle } from './ci_artifacts.mjs';

const run = { repository: 'MorningFrog/agent-job-relay', repo_id: '1', run_id: '13', commit: 'a'.repeat(40) };
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function rewrite(bytes, change) {
  const size = bytes.readUInt32BE(8), meta = JSON.parse(bytes.subarray(12, 12 + size));
  change(meta); const json = Buffer.from(JSON.stringify(meta));
  const prefix = Buffer.from(bytes.subarray(0, 12)); prefix.writeUInt32BE(json.length, 8);
  return Buffer.concat([prefix, json, bytes.subarray(12 + size)]);
}

test('CI identity requires exact repository/run/commit fields', () => {
  const env = { GITHUB_ACTIONS: 'true', GITHUB_REPOSITORY: run.repository, GITHUB_REPOSITORY_ID: run.repo_id, GITHUB_RUN_ID: run.run_id, GITHUB_SHA: run.commit };
  assert.deepEqual(identity(env), run);
  for (const key of Object.keys(env)) assert.throws(() => identity({ ...env, [key]: '' }));
  assert.throws(() => identity({ ...env, GITHUB_SHA: '../other' }));
});

test('frozen byte inventory rejects other runs, producers, corrupt bytes and truncation before extraction', t => {
  const base = join(dirname(fileURLToPath(import.meta.url)), '../.tmp'); mkdirSync(base, { recursive: true });
  const temporary = mkdtempSync(join(base, 'ci-artifacts-')); t.after(() => rmSync(temporary, { recursive: true, force: true }));
  const files = { 'agent-job-relay': Buffer.from('exact-native-image') };
  const bytes = packBundle(run, 'linux-native', files);
  assert.deepEqual(inspectBundle(bytes, run, 'linux-native'), files);
  extractBundle(bytes, run, 'linux-native', join(temporary, 'complete'));
  assert.deepEqual(readFileSync(join(temporary, 'complete/agent-job-relay')), files['agent-job-relay']);
  assert.throws(() => extractBundle(bytes, run, 'linux-native', join(temporary, 'complete')));
  const corrupted = Buffer.from(bytes); corrupted[corrupted.length - 1] ^= 1;
  const bad = [corrupted, bytes.subarray(0, bytes.length - 1), Buffer.concat([bytes, Buffer.from('extra')]),
    rewrite(bytes, m => m.identity.repo_id = '9'), rewrite(bytes, m => m.identity.repository = 'other/repo'),
    rewrite(bytes, m => m.identity.run_id = '14'), rewrite(bytes, m => m.identity.commit = 'b'.repeat(40)),
    rewrite(bytes, m => m.producer = 'windows'), rewrite(bytes, m => m.files[0].name = '../escape'),
    rewrite(bytes, m => m.files[0].name = 'NUL.txt'), rewrite(bytes, m => m.files.push(m.files[0]))];
  for (const value of bad) {
    assert.throws(() => extractBundle(value, run, 'linux-native', join(temporary, 'rejected')));
    assert(!existsSync(join(temporary, 'rejected')));
  }
  assert.throws(() => packBundle(run, 'linux-native', { 'case.txt': Buffer.from('a'), 'CASE.txt': Buffer.from('b') }));
});

test('native metadata and development packages cannot substitute another revision or formal candidate', () => {
  const image = Buffer.from('windows-native');
  const native = { 'agent-job-relay.exe': image, 'windows-native.json': Buffer.from(JSON.stringify({ format: 1, platform: 'win32-x64', revision: run.commit, sha256: sha(image) })) };
  const bytes = packBundle(run, 'windows-native', native);
  assert.deepEqual(inspectBundle(bytes, run, 'windows-native'), native);
  native['windows-native.json'] = Buffer.from(JSON.stringify({ format: 1, platform: 'win32-x64', revision: 'b'.repeat(40), sha256: sha(image) }));
  assert.throws(() => inspectBundle(packBundle(run, 'windows-native', native), run, 'windows-native'));
  const meta = { development: true, platforms: ['linux-x64', 'win32-x64'], runtime: { tarball: 'runtime.tgz' }, opencode: { tarball: 'opencode.tgz' }, marketplace: 'market.zip' };
  const packages = { 'release.json': Buffer.from(JSON.stringify(meta)), 'runtime.tgz': Buffer.from('runtime'), 'opencode.tgz': Buffer.from('plugin'), 'market.zip': Buffer.from('market') };
  packages.SHA256SUMS = Buffer.from(Object.entries(packages).map(([n, data]) => sha(data) + '  ' + n + '\n').join(''));
  inspectBundle(packBundle(run, 'ci-packages', packages), run, 'ci-packages');
  assert.throws(() => inspectBundle(packBundle(run, 'release-packages', packages), run, 'release-packages'));
  const { development, ...formal } = meta;
  const release = { ...packages, 'release.json': Buffer.from(JSON.stringify(formal)) };
  delete release.SHA256SUMS;
  release.SHA256SUMS = Buffer.from(Object.entries(release).map(([n, data]) => sha(data) + '  ' + n + '\n').join(''));
  inspectBundle(packBundle(run, 'release-packages', release), run, 'release-packages');
  assert.throws(() => inspectBundle(packBundle(run, 'ci-packages', release), run, 'ci-packages'));
  for (const bad of [{ ...packages, 'extra.txt': Buffer.from('foreign') }, { ...packages, SHA256SUMS: Buffer.from('bad') }, { ...packages, 'release.json': Buffer.from(JSON.stringify({ ...meta, development: false })) }]) assert.throws(() => inspectBundle(packBundle(run, 'ci-packages', bad), run, 'ci-packages'));
});

test('sealed candidate extraction requires the package inventory and the original preparation run', t => {
  const base = join(dirname(fileURLToPath(import.meta.url)), '../.tmp');
  mkdirSync(base, { recursive: true });
  const temporary = mkdtempSync(join(base, 'ci-candidate-'));
  t.after(() => rmSync(temporary, { recursive: true, force: true }));
  const candidate = {
    format: 1, run_id: run.run_id, repository: run.repository,
    revision: run.commit, workflow: '.github/workflows/release.yml',
  };
  const meta = {
    platforms: ['linux-x64', 'win32-x64'],
    runtime: { tarball: 'runtime.tgz' }, opencode: { tarball: 'plugin.tgz' }, marketplace: 'market.zip',
  };
  const files = {
    'runtime.tgz': Buffer.from('runtime'), 'plugin.tgz': Buffer.from('plugin'), 'market.zip': Buffer.from('market'),
    'release.json': Buffer.from(JSON.stringify(meta)), 'release-notes.md': Buffer.from('Reviewed notes'),
  };
  function bundle(seal) {
    const content = { ...files, 'candidate.json': Buffer.from(JSON.stringify(seal)) };
    content.SHA256SUMS = Buffer.from(Object.entries(content).map(([name, data]) => `${sha(data)}  ${name}\n`).join(''));
    return packBundle(run, 'release-candidate', content);
  }
  const bytes = bundle(candidate);
  extractBundle(bytes, run, 'release-candidate', join(temporary, 'accepted'));
  for (const [object, source] of [
    [bytes, { ...run, run_id: '14' }],
    [bundle({ ...candidate, run_id: '14' }), run],
    [bundle({ ...candidate, workflow: '.github/workflows/ci.yml' }), run],
  ]) {
    const destination = join(temporary, 'rejected');
    assert.throws(() => extractBundle(object, source, 'release-candidate', destination));
    assert(!existsSync(destination));
  }
});
