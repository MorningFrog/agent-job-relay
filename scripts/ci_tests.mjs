// Run every Cargo all-target test suite with bounded parallelism. No result cache.
import fs from 'node:fs';
import path from 'node:path';
import {spawn, spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

export function targetsFrom(messages) {
  if (!messages.some(m => m.reason === 'build-finished' && m.success === true))
    throw new Error('Cargo did not confirm a complete test build');
  const targets = new Map();
  for (const m of messages) {
    if (m.reason !== 'compiler-artifact' || !m.executable) continue;
    const kind = m.target.kind;
    // Cargo also builds the ordinary CLI used by integration fixtures. Other
    // non-test executables may be custom harnesses: refuse rather than omit one.
    if (!m.profile?.test) {
      if (kind.length === 1 && kind[0] === 'bin') continue;
      throw new Error('Unsupported Cargo executable; no suite was skipped');
    }
    if (kind.length !== 1 || !['lib', 'bin', 'test', 'example', 'bench'].includes(kind[0]))
      throw new Error('Unsupported Cargo test target; no suite was skipped');
    const selector = kind[0] === 'lib' ? ['--lib'] : ['--' + kind[0], m.target.name];
    const args = ['--package', m.package_id, ...selector];
    const key = JSON.stringify(args);
    if (targets.has(key) && targets.get(key).executable !== m.executable)
      throw new Error('Conflicting Cargo test artifacts');
    targets.set(key, {name: kind[0] + ':' + m.target.name, args, executable: m.executable});
  }
  if (!targets.size) throw new Error('Cargo produced no test suites');
  return [...targets.values()];
}

export async function runSuites(targets, jobs, run) {
  if (!Number.isInteger(jobs) || jobs < 1 || jobs > 8) throw new Error('Test jobs must be 1..8');
  let next = 0;
  const results = new Array(targets.length);
  await Promise.all(Array.from({length: Math.min(jobs, targets.length)}, async () => {
    while (next < targets.length) {
      const index = next++;
      try { results[index] = await run(targets[index], index); }
      catch (error) { results[index] = {name: targets[index].name, exit: 1, error: String(error)}; }
    }
  }));
  return results;
}

export async function main(options = process.argv.slice(2)) {
  let jobs = Number(process.env.AJR_CI_TEST_JOBS || 2), toolchain = [];
  for (const option of options) {
    if (/^--jobs=[1-8]$/.test(option)) jobs = Number(option.slice(7));
    else if (/^--toolchain=\d+\.\d+\.\d+$/.test(option)) toolchain = ['+' + option.slice(12)];
    else throw new Error('Usage: node scripts/ci_tests.mjs [--jobs=1..8] [--toolchain=x.y.z]');
  }
  if (!Number.isInteger(jobs) || jobs < 1 || jobs > 8) throw new Error('Test jobs must be 1..8');
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const tmp = path.join(root, '.tmp'); fs.mkdirSync(tmp, {recursive: true});
  const logs = fs.mkdtempSync(path.join(tmp, 'ci-tests-'));
  const start = performance.now();
  const build = spawnSync('cargo', [...toolchain, 'test', '--all-targets', '--locked', '--no-run', '--message-format=json'],
    {cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024});
  fs.writeFileSync(path.join(logs, 'build.jsonl'), build.stdout || '');
  fs.writeFileSync(path.join(logs, 'build.stderr'), build.stderr || '');
  process.stderr.write(build.stderr || '');
  if (build.error || build.status !== 0) throw new Error('Cargo test build failed; inspect ' + logs);
  const builtAt = performance.now();
  const targets = targetsFrom(build.stdout.trim().split('\n').filter(Boolean).map(line => JSON.parse(line)));
  // Cargo emits artifacts in completion order, which is unstable even on warm
  // builds. Start the known long-lived isolated fixtures before short suites.
  // This is only scheduling priority: every newly discovered suite still runs.
  const first = ['test:plugin', 'test:goal_wait', 'test:onboarding', 'test:logs', 'test:legacy_upgrade'];
  const rank = target => first.includes(target.name) ? first.indexOf(target.name) : first.length;
  targets.sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
  console.log(`Running all ${targets.length} Cargo suites, ${jobs} at a time; logs: ${logs}`);
  const results = await runSuites(targets, jobs, (target, index) => new Promise((resolve, reject) => {
    const file = path.join(logs, `${index}.log`), fd = fs.openSync(file, 'w');
    const started = performance.now();
    // Cargo retains its official runtime environment and current-workspace
    // variables. A no-run build precedes the readers; Cargo locks its own output.
    const child = spawn('cargo', [...toolchain, 'test', '--locked', ...target.args],
      {cwd: root, env: process.env, stdio: ['ignore', fd, fd]});
    fs.closeSync(fd);
    child.once('error', reject);
    child.once('close', (code, signal) => {
      const result = {name: target.name, exit: code ?? 1, signal,
        seconds: Math.round((performance.now() - started) / 10) / 100, log: file};
      console.log(JSON.stringify(result));
      if (result.exit !== 0) process.stderr.write(fs.readFileSync(file, 'utf8'));
      resolve(result);
    });
  }));
  const report = {seconds: Math.round((performance.now() - start) / 10) / 100,
    build_seconds: Math.round((builtAt - start) / 10) / 100,
    test_seconds: Math.round((performance.now() - builtAt) / 10) / 100, jobs, results};
  fs.writeFileSync(path.join(logs, 'summary.json'), JSON.stringify(report, null, 2) + '\n');
  if (results.some(r => r.exit !== 0)) throw new Error('Cargo suite failed; inspect ' + logs);
  console.log(`All ${targets.length} Cargo suites passed in ${report.seconds}s`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}
