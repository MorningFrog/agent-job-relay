// Read only: accepted run metadata and immutable candidate; no build or upload.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { requestExchange, transfer, inspectBundle, extractBundle } from './ci_artifacts.mjs';
import { inspectRelease, PRODUCERS } from './release_artifacts.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function check(ok, message) { if (!ok) throw new Error(message); }
export async function fetchCandidate(requestPath, destination, statusPath) {
  const wanted = JSON.parse(readFileSync(requestPath));
  check(/^[1-9][0-9]{0,11}$/.test(wanted.repo_id) && /^[1-9][0-9]{0,11}$/.test(wanted.pipeline), 'Invalid requested candidate run');
  const bytes = await requestExchange(`/metadata/v1/${wanted.repo_id}/${wanted.pipeline}`, undefined, 'AJR_PUBLISH');
  check(bytes.length <= 1024 * 1024, 'Candidate status response exceeds bound');
  const metadata = JSON.parse(bytes), p = metadata.pipeline;
  check(metadata.provider === 'woodpecker' && metadata.server === wanted.server &&
    String(p.repo_id) === wanted.repo_id && String(p.number) === wanted.pipeline &&
    p.status === 'success' && p.event === 'manual' && /^[0-9a-f]{40}$/.test(p.commit) &&
    /^[1-9][0-9]{0,11}$/.test(String(p.created)), 'Candidate lacks a matching successful manual run');
  const run = { repository: 'MorningFrog/agent-job-relay', repo_id: wanted.repo_id, pipeline: wanted.pipeline,
    created: String(p.created), commit: p.commit, server: metadata.server };
  const object = await transfer(run, 'release-candidate', undefined, 'AJR_PUBLISH');
  const files = inspectBundle(object, run, 'release-candidate', PRODUCERS);
  const candidate = JSON.parse(files['candidate.json']);
  check(['prepare', 'rehearse'].includes(candidate.execution?.task) && ['latest', 'next'].includes(candidate.execution?.channel), 'Invalid candidate policy');
  const context = { format: 1, run, branch: p.branch, task: candidate.execution.task, channel: candidate.execution.channel };
  inspectRelease(object, context, 'release-candidate');
  extractBundle(object, run, 'release-candidate', destination, PRODUCERS);
  writeFileSync(statusPath, JSON.stringify(metadata, null, 2) + '\n', { mode: 0o600, flag: 'wx' });
  console.log('Downloaded the exact successful preparation seal; publication eligibility is checked separately.');
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.length !== 0 && args.length !== 3) throw new Error('Expected request, destination and status paths');
  fetchCandidate(...(args.length ? args : [join(root, '.tmp/publication-request.json'), join(root, '.tmp/publication-candidate'), join(root, '.tmp/publication-status.json')])).catch(error => { console.error(error.message); process.exitCode = 1; });
}
