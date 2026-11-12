// Desktop launch flow (ui/loading.js): the launch screen waits for the law index to warm, through
// the `law-ready` event or `features.lawReady` polling, for at most 60 s; and the desktop
// index.html has no gate and shows the launch screen from the first paint.
// Run: `npm test` (node --test, no dependencies).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

globalThis.document = { querySelector: () => null }; // ui/dom.js reads it lazily; never used here
const { loadingText, waitForLaw } = await import('../ui/loading.js');
const { desktopIndex } = await import('../scripts/desktop-index.mjs');

/** A fake clock and api: `configs` are the get_config results in order (the last repeats). */
function fakes(configs) {
  let t = 0;
  const calls = { getConfig: 0, unlistened: 0 };
  let fire = null;
  const api = {
    getConfig: async () => configs[Math.min(calls.getConfig++, configs.length - 1)],
    onLawReady: async (fn) => { fire = fn; return () => { calls.unlistened += 1; }; },
  };
  const clock = { now: () => t, sleep: async (ms) => { t += ms; } };
  return { api, clock, calls, fire: () => fire() };
}

const cfg = (lawReady, lawCount = 10870) => ({ features: { statutes: true, lawCount, lawReady } });

test('stays on the launch screen while lawReady is false, then goes Home once it is true', async () => {
  const { api, clock, calls } = fakes([cfg(false), cfg(false), cfg(true)]);
  const counts = [];

  const result = await waitForLaw(api, { ...clock, onCount: (n) => counts.push(n) });

  assert.deepEqual(result, { timedOut: false });
  assert.equal(calls.getConfig, 3);
  assert.deepEqual(counts, [10870, 10870, 10870]);
  assert.equal(calls.unlistened, 1);
});

test('the law-ready event ends the wait without another poll', async () => {
  const f = fakes([cfg(false)]);
  f.clock.sleep = () => new Promise(() => {}); // only the event can wake the loop

  const waiting = waitForLaw(f.api, f.clock);
  await new Promise((r) => setTimeout(r, 0));
  f.fire();

  assert.deepEqual(await waiting, { timedOut: false });
  assert.equal(f.calls.getConfig, 1);
});

test('gives up after 60 s and reports the timeout', async () => {
  const { api, clock, calls } = fakes([cfg(false)]);

  const result = await waitForLaw(api, clock);

  assert.deepEqual(result, { timedOut: true });
  assert.equal(calls.getConfig, 121); // t = 0, 500, ..., 60 000 ms
});

test('a config without lawReady (the web build) never waits', async () => {
  const { api, clock, calls } = fakes([{ model: 'm', models: [] }]);

  assert.deepEqual(await waitForLaw(api, clock), { timedOut: false });
  assert.equal(calls.getConfig, 1);
});

test('a failing get_config keeps polling until the timeout', async () => {
  const { api, clock } = fakes([cfg(false)]);
  api.getConfig = async () => { throw new Error('not yet'); };

  assert.deepEqual(await waitForLaw(api, clock), { timedOut: true });
});

test('the loading text names the law count when known', () => {
  assert.equal(loadingText(10870), 'Loading 10,870 Philippine laws…');
  assert.equal(loadingText(undefined), 'Loading Philippine laws…');
  assert.equal(loadingText(10870, { statutes: true }), 'Loading 10,870 Philippine laws…');
});

test('the loading text names Supreme Court decisions when config expects cases', () => {
  assert.equal(loadingText(12289, { cases: true, caseCount: 31180 }), 'Loading Philippine laws and Supreme Court decisions…');
  assert.equal(loadingText(undefined, { cases: true }), 'Loading Philippine laws and Supreme Court decisions…');
});

test('onCount receives the config features next to the law count', async () => {
  const withCases = { features: { statutes: true, lawCount: 12289, lawReady: true, cases: true, caseCount: 31180 } };
  const { api, clock } = fakes([withCases]);
  const seen = [];

  await waitForLaw(api, { ...clock, onCount: (n, f) => seen.push([n, f]) });

  assert.deepEqual(seen, [[12289, withCases.features]]);
});

test('the desktop index.html has no gate and shows the launch screen at once', () => {
  const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8');

  const desktop = desktopIndex(html);

  assert.ok(html.includes('id="gate"'));
  assert.ok(!desktop.includes('id="gate"') && !desktop.includes('id="otp"'));
  assert.ok(desktop.includes('<div id="loading" class="loading" role="status" aria-live="polite">'));
  assert.ok(desktop.includes('id="app"'));
});
