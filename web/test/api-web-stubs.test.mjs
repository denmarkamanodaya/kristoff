// The web api.js keeps export parity with the desktop key and settings functions (Phase 8): each
// stub rejects UNSUPPORTED without any network call.
import { test } from 'node:test';
import assert from 'node:assert/strict';

let fetches = 0;
globalThis.fetch = async () => { fetches += 1; throw new Error('no network in this test'); };
const api = await import('../services/api.js');

test('the desktop-only key and settings functions reject UNSUPPORTED on the web, with no fetch', async () => {
  for (const name of ['getKeyStatus', 'setApiKey', 'removeApiKey', 'getSettings', 'updateSettings']) {
    const err = await api[name]('x').catch((e) => e);
    assert.ok(err instanceof api.ApiError, name);
    assert.equal(err.code, 'UNSUPPORTED', name);
  }
  assert.equal(fetches, 0);
});
