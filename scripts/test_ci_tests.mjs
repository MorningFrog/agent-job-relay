import test from 'node:test';
import assert from 'node:assert/strict';
import {targetsFrom, runSuites} from './ci_tests.mjs';

const artifact = (kind, name, testProfile = true) => ({reason: 'compiler-artifact',
  package_id: 'fixture@1.0.0', target: {kind: [kind], name}, profile: {test: testProfile}, executable: '/fixture/' + name});
const finished = {reason: 'build-finished', success: true};

test('all-target discovery preserves all suites and excludes normal binary artifacts', () => {
  const records = [artifact('bin', 'app', false), ...['lib', 'bin', 'test', 'example', 'bench'].map(k => artifact(k, k)), finished];
  const targets = targetsFrom(records);
  assert.equal(targets.length, 5);
  assert.deepEqual(targets[0].args, ['--package', 'fixture@1.0.0', '--lib']);
  assert.deepEqual(targets[2].args, ['--package', 'fixture@1.0.0', '--test', 'test']);
  assert.throws(() => targetsFrom(records.slice(0, -1)), /complete test build/);
  assert.throws(() => targetsFrom([finished]), /no test suites/);
  assert.throws(() => targetsFrom([artifact('custom', 'bad'), finished]), /Unsupported/);
  assert.throws(() => targetsFrom([artifact('example', 'custom-harness', false), finished]), /Unsupported/);
  assert.throws(() => targetsFrom([artifact('test', 'x'), {...artifact('test', 'x'), executable: '/other'}, finished]), /Conflicting/);
});

test('bounded execution overlaps suites, retains failures and attempts every target', async () => {
  const targets = Array.from({length: 7}, (_, i) => ({name: 'test:' + i}));
  let active = 0, peak = 0, release;
  const overlap = new Promise(resolve => { release = resolve; });
  const attempted = [];
  const results = await runSuites(targets, 2, async (target, index) => {
    attempted.push(target.name); peak = Math.max(peak, ++active);
    if (active === 2) release();
    await overlap;
    active--;
    if (index === 3) throw new Error('spawn failure');
    return {name: target.name, exit: index === 4 ? 1 : 0};
  });
  assert.equal(peak, 2);
  assert.deepEqual(attempted.sort(), targets.map(t => t.name).sort());
  assert.equal(results.length, targets.length);
  assert.equal(results[3].exit, 1);
  assert.match(results[3].error, /spawn failure/);
  assert.equal(results[4].exit, 1);
  await assert.rejects(runSuites(targets, 0, () => {}), /1..8/);
});
