// Tests for ui/settings-flow.js (Phase 8 bring-your-own-key rules) and the theme-key migration in
// services/prefs.js (LexPH rename).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CAP_BOUNDS, askBlocked, checkDailyCap, checkKeyInput, isKeyError, keyErrorText, keyLine, shouldOnboard } from '../ui/settings-flow.js';

test('the key line shows the masked hint, a bare tick, or no key', () => {
  assert.equal(keyLine({ configured: true, hint: 'ab12' }), 'sk-ant-…ab12 ✓');
  assert.equal(keyLine({ configured: true, hint: null }), 'Saved ✓');
  assert.equal(keyLine({ configured: false, hint: null }), 'No key saved');
  assert.equal(keyLine(null), 'No key saved');
});

test('a pasted key is trimmed, and empty, foreign or broken keys are refused before sending', () => {
  assert.deepEqual(checkKeyInput('  sk-ant-api03-xyz \n'), { key: 'sk-ant-api03-xyz' });
  assert.ok(checkKeyInput('').error);
  assert.ok(checkKeyInput('sk-proj-123').error);
  assert.ok(checkKeyInput('sk-ant-abc def').error);
  assert.ok(checkKeyInput(`sk-ant-${'a'.repeat(300)}`).error);
});

test('the daily limit accepts whole numbers inside the bounds only', () => {
  const b = { min: 1, max: 1000 };
  assert.deepEqual(checkDailyCap('100', b), { value: 100 });
  assert.deepEqual(checkDailyCap(' 1 ', b), { value: 1 });
  assert.deepEqual(checkDailyCap('1000', b), { value: 1000 });
  for (const bad of ['0', '1001', '2.5', '-3', 'ten', '']) assert.ok(checkDailyCap(bad, b).error, bad);
});

test('the daily limit range is the contract range 1..10000', () => {
  assert.deepEqual({ ...CAP_BOUNDS }, { min: 1, max: 10000 });
  assert.deepEqual(checkDailyCap('10000', CAP_BOUNDS), { value: 10000 });
  assert.ok(checkDailyCap('10001', CAP_BOUNDS).error);
});

test('Settings opens by itself only when AI is not ready and no key is saved', () => {
  assert.equal(shouldOnboard({ aiReady: false }, { configured: false }), true);
  assert.equal(shouldOnboard({ aiReady: false }, { configured: true }), false); // rejected key: composer prompt instead
  assert.equal(shouldOnboard({ aiReady: true }, { configured: false }), false);
  assert.equal(shouldOnboard({}, { configured: false }), false); // build without BYOK
  assert.equal(shouldOnboard({ aiReady: false }, null), false); // status unreadable
});

test('asking is blocked only when aiReady is explicitly false', () => {
  assert.equal(askBlocked({ aiReady: false }), true);
  assert.equal(askBlocked({ aiReady: true }), false);
  assert.equal(askBlocked({}), false);
  assert.equal(askBlocked(undefined), false);
});

test('KEY_MISSING and KEY_INVALID get friendly text and the Settings button; other codes do not', () => {
  assert.equal(isKeyError({ code: 'KEY_MISSING' }), true);
  assert.equal(isKeyError({ code: 'KEY_INVALID' }), true);
  assert.equal(isKeyError({ code: 'INTERNAL' }), false);
  assert.match(keyErrorText('KEY_MISSING'), /Add your Claude API key in Settings/);
  assert.match(keyErrorText('KEY_INVALID'), /rejected.*Settings/);
  assert.equal(keyErrorText('INTERNAL'), null);
});

test('the theme moves from the pre-rename key to lexph.theme once', async () => {
  const store = new Map([['kristoff.theme', 'light']]);
  globalThis.localStorage = {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => store.set(k, String(v)),
    removeItem: (k) => store.delete(k),
  };
  const prefs = await import('../services/prefs.js');

  assert.equal(prefs.getTheme(), 'light');
  assert.deepEqual([...store.entries()], [['lexph.theme', 'light']]);
  prefs.setTheme('dark');
  assert.equal(prefs.getTheme(), 'dark');
  assert.equal(store.get('lexph.theme'), 'dark');
});
