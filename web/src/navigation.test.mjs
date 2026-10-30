import assert from 'node:assert/strict';
import test from 'node:test';
import {consumePanelNavigation, subscribePanelNavigation} from './navigation.js';

function browserAt(url, savedToken) {
  const browser = new EventTarget();
  let current = new URL(url);
  const storage = new Map(savedToken ? [['ajr-token', savedToken]] : []);
  Object.defineProperty(browser, 'location', {get: () => current});
  browser.sessionStorage = {getItem: key => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value)};
  browser.history = {replaceState: (_state, _title, path) => {current = new URL(path, current);}};
  browser.navigate = hash => {current.hash = hash; browser.dispatchEvent(new Event('hashchange'));};
  return browser;
}

test('initial task navigation consumes credentials without losing the requested job', () => {
  const browser = browserAt('http://127.0.0.1:12345/?view=tasks#token=first&job=job-a');
  assert.deepEqual(consumePanelNavigation(browser), {jobId: 'job-a', token: 'first'});
  assert.equal(browser.location.href, 'http://127.0.0.1:12345/?view=tasks');
  assert.equal(browser.sessionStorage.getItem('ajr-token'), 'first');
});

test('one live page receives repeated task, credential and overview navigation', () => {
  const browser = browserAt('http://127.0.0.1:12345/#token=first&job=job-a');
  consumePanelNavigation(browser);
  const received = [];
  const unsubscribe = subscribePanelNavigation(value => received.push(value), browser);
  for (const hash of ['#token=first&job=job-b', '#token=first&job=job-b', '#job=job-c', '#token=renewed&job=job-c', '#token=renewed']) {
    browser.navigate(hash);
    assert.equal(browser.location.hash, '');
  }
  assert.deepEqual(received, [
    {jobId: 'job-b', token: 'first'},
    {jobId: 'job-b', token: 'first'},
    {jobId: 'job-c', token: 'first'},
    {jobId: 'job-c', token: 'renewed'},
    {jobId: null, token: 'renewed'},
  ]);
  unsubscribe();
  browser.navigate('#token=unused&job=job-d');
  assert.equal(received.length, 5);
  assert.equal(browser.sessionStorage.getItem('ajr-token'), 'renewed');
});

test('an unauthenticated page can receive a fresh access URL in place', () => {
  const browser = browserAt('http://127.0.0.1:12345/');
  assert.deepEqual(consumePanelNavigation(browser), {jobId: null, token: ''});
  const received = [];
  const unsubscribe = subscribePanelNavigation(value => received.push(value), browser);
  browser.navigate('#token=fresh&job=job-a');
  assert.deepEqual(received, [{jobId: 'job-a', token: 'fresh'}]);
  assert.equal(browser.location.hash, '');
  unsubscribe();
});
