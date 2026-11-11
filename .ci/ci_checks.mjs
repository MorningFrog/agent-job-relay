// Portable output-free check reuse. Node >= 20, Git. No account credentials.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import https from 'node:https';
import {spawnSync} from 'node:child_process';

const policyPath = '.ci/checks.json';
const clientPath = '.ci/ci_checks.mjs';
const [name, ...options] = process.argv.slice(2);
if (!/^[a-z][a-z0-9-]{0,47}$/.test(name || '') || options.some(v => v !== '--force'))
  throw new Error('Usage: node .ci/ci_checks.mjs <check> [--force]');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const check = policy.checks?.[name];
if (policy.format !== 1 || !check || !Array.isArray(check.command) || !check.command.length ||
    check.command.some(v => typeof v !== 'string' || !v || v.includes('\0')) ||
    !Array.isArray(check.inputs) || !check.inputs.length)
  throw new Error('Invalid check policy');

// Python's canonical JSON includes spaces and ASCII escapes. Preserve exactly
// that encoding so host and both native clients compute the same input key.
function canonical(value) {
  if (Array.isArray(value)) return '[' + value.map(canonical).join(', ') + ']';
  if (value && typeof value === 'object') return '{' + Object.keys(value).sort().map(
    key => canonical(key) + ': ' + canonical(value[key])).join(', ') + '}';
  return JSON.stringify(value).replace(/[\u007f-\uffff]/g,
    c => '\\u' + c.charCodeAt(0).toString(16).padStart(4, '0'));
}
function git(...args) {
  const r = spawnSync('git', args, {encoding: 'utf8', timeout: 15000, maxBuffer: 32 * 1024 * 1024});
  if (r.error || r.status !== 0) throw new Error('Check source unavailable');
  return r.stdout;
}
// Match Python fnmatchcase: * crosses slash; [] classes are deliberately
// rejected in project policies to keep both implementations unambiguous.
function matches(file, pattern) {
  const expression = [...pattern].map(c => c === '*' ? '.*' : c === '?' ? '.' :
    c.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('');
  return new RegExp('^' + expression + '$', 's').test(file);
}
const selected = file => file === policyPath || file === clientPath || file.startsWith('.woodpecker/') ||
  check.inputs.some(pattern => matches(file, pattern));
function fingerprint(revision) {
  const files = [];
  for (const record of git('ls-tree', '-rz', revision).split('\0').filter(Boolean)) {
    const separator = record.indexOf('\t');
    const [mode, kind, digest] = record.slice(0, separator).split(' ');
    const file = record.slice(separator + 1);
    if (!selected(file)) continue;
    if (kind !== 'blob' || !['100644', '100755'].includes(mode)) throw new Error('External check input');
    files.push([file, mode, digest]);
  }
  if (![policyPath, clientPath].every(required => files.some(([file]) => file === required)))
    throw new Error('Commit the check policy and client first');
  // UTF-8 order preserves Unicode scalar order, including non-BMP names.
  files.sort((a,b) => Buffer.compare(Buffer.from(a[0]), Buffer.from(b[0])));
  return crypto.createHash('sha256').update(canonical({policy, check: name, files})).digest('hex');
}
function lookup(number, hash) {
  const origin = new URL(policy.endpoint);
  if (origin.protocol !== 'https:' || origin.username || origin.password || origin.search || origin.hash ||
      !['', '/'].includes(origin.pathname)) throw new Error('Invalid check endpoint');
  const url = new URL(`/checks/${policy.repository_id}/${number}/${name}/${hash}`, origin);
  return new Promise((resolve, reject) => {
    const request = https.get(url, {ca: policy.ca_pem || undefined}, response => {
      if (response.statusCode !== 200) {
        response.resume(); reject(new Error('Check evidence unavailable')); return;
      }
      let data = '';
      response.setEncoding('utf8');
      response.on('data', part => {
        data += part;
        if (data.length > 16384) response.destroy(new Error('Oversize evidence'));
      });
      response.on('error', reject);
      response.on('end', () => {
        try { resolve(JSON.parse(data)); } catch (error) { reject(error); }
      });
    });
    const timer = setTimeout(() => request.destroy(new Error('Evidence timeout')), 5000);
    request.on('close', () => clearTimeout(timer));
    request.on('error', reject);
  });
}
const env = process.env, force = options.includes('--force') || env.CI_CHECKS_FORCE === 'true';
const record = {check: name, state: 'running', reused: false, reason: force ? 'forced' : 'not_eligible'};
try {
  const revision = git('rev-parse', 'HEAD').trim();
  record.hash = fingerprint(revision);
  const dirty = (git('diff', '--name-only', '-z', 'HEAD') + git('ls-files', '--others', '--exclude-standard', '-z'))
    .split('\0').filter(Boolean).some(selected);
  if (!force && !dirty && /^[1-9][0-9]*$/.test(env.CI_PIPELINE_NUMBER || '') &&
      env.CI_REPO_ID === String(policy.repository_id) && env.CI_REPO === policy.repository &&
      env.CI_COMMIT_SHA === revision && env.CI_COMMIT_BRANCH === policy.branch &&
      ['push', 'manual'].includes(env.CI_PIPELINE_EVENT) &&
      policy.valid_after <= Date.now()/1000 && Date.now()/1000 < policy.expires) {
    const result = await lookup(env.CI_PIPELINE_NUMBER, record.hash);
    record.reason = result.reason || 'no_match';
    if (result.reused === true) {
      const source = result.source;
      if (result.hash !== record.hash || !source || !Number.isSafeInteger(source.pipeline) ||
          source.pipeline <= 0 || source.pipeline >= Number(env.CI_PIPELINE_NUMBER) ||
          !/^[0-9a-f]{40}$/.test(source.commit) ||
          !check.steps.some(s => s.workflow === source.workflow && s.step === source.step))
        throw new Error('Unbound check evidence');
      Object.assign(record, {state: 'success', reused: true, source});
    }
  }
} catch {
  record.reason = 'evidence_unavailable';
}
let exitCode = 0;
if (!record.reused) {
  console.log(JSON.stringify({...record, action: 'execute'}));
  const result = spawnSync(check.command[0], check.command.slice(1), {stdio: 'inherit'});
  // An interrupted/failed launch must never look like a successful check.
  exitCode = result.status ?? 1;
  Object.assign(record, {state: exitCode === 0 ? 'success' : 'failure', exit_code: exitCode});
}
const directory = '.tmp/ci-checks';
fs.mkdirSync(directory, {recursive: true});
const temporary = path.join(directory, `${name}.${crypto.randomUUID()}.tmp`);
fs.writeFileSync(temporary, JSON.stringify(record) + '\n', {flag: 'wx'});
fs.renameSync(temporary, path.join(directory, name + '.json'));
console.log(JSON.stringify(record));
process.exitCode = exitCode;
