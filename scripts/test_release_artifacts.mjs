import { test } from 'node:test';
import assert from 'node:assert/strict';
import { packBundle, inspectBundle } from './ci_artifacts.mjs';
import { execution, inspectRelease, PRODUCERS, hash } from './release_artifacts.mjs';
const env = { CI: 'woodpecker', CI_PIPELINE_EVENT: 'manual', CI_REPO: 'MorningFrog/agent-job-relay',
  CI_REPO_ID: '1', CI_PIPELINE_NUMBER: '42', CI_PIPELINE_CREATED: '1700000000', CI_COMMIT_SHA: 'a'.repeat(40),
  EXPECTED_RELEASE_REVISION: 'a'.repeat(40), CI_COMMIT_REF: 'refs/heads/main', CI_COMMIT_BRANCH: 'main',
  CI_PIPELINE_URL: 'https://ci.example.test/repos/1/pipeline/42', AJR_RELEASE_TASK: 'prepare', AJR_RELEASE_CHANNEL: 'next' };
const context = execution(env);
function bundle(kind, files, selected = context) { return packBundle(selected.run, kind, files, PRODUCERS); }
function packages(development = false) {
  const meta = { version: '1.2.3', platforms: ['linux-x64', 'win32-x64'],
    runtime: { tarball: 'runtime.tgz' }, opencode: { tarball: 'opencode.tgz' }, marketplace: 'marketplace.zip' };
  if (development) meta.development = true;
  const files = { 'release.json': Buffer.from(JSON.stringify(meta)), 'runtime.tgz': Buffer.from('runtime'),
    'opencode.tgz': Buffer.from('opencode'), 'marketplace.zip': Buffer.from('marketplace') };
  files.SHA256SUMS = Buffer.from(Object.entries(files).map(([n, data]) => hash(data) + '  ' + n).join('\n') + '\n');
  return files;
}
test('manual release context rejects automatic events, mixed sources and implicit modes', () => {
  for (const change of [{ CI_PIPELINE_EVENT: 'push' }, { CI_PIPELINE_EVENT: 'tag' }, { CI_PIPELINE_EVENT: 'pull_request' },
    { EXPECTED_RELEASE_REVISION: 'b'.repeat(40) }, { AJR_RELEASE_TASK: '' }, { AJR_RELEASE_CHANNEL: '' },
    { CI_COMMIT_REF: 'refs/tags/v1.2.3' }, { CI_COMMIT_BRANCH: 'feature', CI_COMMIT_REF: 'refs/heads/feature' },
    { CI_PIPELINE_URL: 'https://password@ci.example.test/repos/1/pipeline/42' }]) assert.throws(() => execution({ ...env, ...change }));
  assert.equal(execution({ ...env, AJR_RELEASE_TASK: 'rehearse', CI_COMMIT_BRANCH: 'feature', CI_COMMIT_REF: 'refs/heads/feature' }).task, 'rehearse');
});
test('manual preparation objects cannot enter the ordinary CI artifact protocol', () => {
  const files = { 'preflight.json': Buffer.from(JSON.stringify({ execution: context })) };
  const data = bundle('release-preflight', files);
  assert.deepEqual(inspectRelease(data, context, 'release-preflight'), files);
  assert.throws(() => inspectBundle(data, context.run, 'release-preflight'));
  for (const selected of [{ ...context, channel: 'latest' }, { ...context, task: 'rehearse' },
    { ...context, run: { ...context.run, pipeline: '43' } }, { ...context, run: { ...context.run, server: 'https://other.test' } }]) {
    assert.throws(() => inspectRelease(data, selected, 'release-preflight'));
  }
  const corrupt = Buffer.from(data); corrupt[corrupt.length - 1] ^= 1;
  assert.throws(() => inspectRelease(corrupt, context, 'release-preflight'));
});
test('development bytes remain development and complete package checksums are mandatory', () => {
  const files = packages(); assert.deepEqual(inspectRelease(bundle('release-packages', files), context, 'release-packages'), files);
  const development = { ...context, task: 'rehearse' }, devFiles = packages(true);
  assert.throws(() => inspectRelease(bundle('release-packages', devFiles), context, 'release-packages'));
  assert.deepEqual(inspectRelease(bundle('release-packages', devFiles, development), development, 'release-packages'), devFiles);
  assert.throws(() => inspectRelease(bundle('release-packages', files, development), development, 'release-packages'));
  for (const changed of [{ ...files, extra: Buffer.from('foreign') }, { ...files, SHA256SUMS: Buffer.from('') },
    { ...files, 'runtime.tgz': Buffer.from('different bytes') }]) assert.throws(() => inspectRelease(bundle('release-packages', changed), context, 'release-packages'));
});
test('native compatibility and installed-package reports are tied to exact bytes and execution', () => {
  const image = Buffer.from('fixture native image');
  const descriptor = { format: 1, platform: 'linux-x64', revision: context.run.commit, sha256: hash(image),
    compatibility: { glibc: '2.34', sha256: hash(image), result: 'passed' } };
  const files = { 'agent-job-relay': image, 'linux-native.json': Buffer.from(JSON.stringify(descriptor)) };
  inspectRelease(bundle('release-linux-native', files), context, 'release-linux-native');
  descriptor.compatibility.sha256 = 'c'.repeat(64);
  assert.throws(() => inspectRelease(bundle('release-linux-native', { ...files, 'linux-native.json': Buffer.from(JSON.stringify(descriptor)) }), context, 'release-linux-native'));
  const report = Buffer.from('{"result":"passed"}');
  const evidence = { format: 1, execution: context, report_sha256: hash(report), packages_sha256: 'b'.repeat(64) };
  const proof = { 'report.json': report, 'evidence.json': Buffer.from(JSON.stringify(evidence)) };
  inspectRelease(bundle('release-windows-report', proof), context, 'release-windows-report');
  evidence.execution = { ...context, task: 'rehearse' };
  assert.throws(() => inspectRelease(bundle('release-windows-report', { ...proof, 'evidence.json': Buffer.from(JSON.stringify(evidence)) }), context, 'release-windows-report'));
});
