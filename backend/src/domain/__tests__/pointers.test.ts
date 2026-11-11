// Pointer tests (stack/versioning.md): each unit's index.ts exports the intended version.
import { expect, it } from 'vitest';
import * as p0 from '../auth/session-token/index.js';
import * as v0 from '../auth/session-token/v1.js';
import * as p1 from '../auth/totp-lockout/index.js';
import * as v1 from '../auth/totp-lockout/v1.js';
import * as p2 from '../auth/verify-totp/index.js';
import * as v2 from '../auth/verify-totp/v1.js';
import * as p3 from '../chats/build-model-turns/index.js';
import * as v3 from '../chats/build-model-turns/v1.js';
import * as p4 from '../chats/map-citation/index.js';
import * as v4 from '../chats/map-citation/v1.js';
import * as p5 from '../keys/index.js';
import * as v5 from '../keys/v1.js';
import * as p6 from '../usage/usage-day/index.js';
import * as v6 from '../usage/usage-day/v1.js';

it('auth/session-token pointer exports v1', () => {
  expect(p0.signSession).toBe(v0.signSession);
});
it('auth/totp-lockout pointer exports v1', () => {
  expect(p1.lockSecondsLeft).toBe(v1.lockSecondsLeft);
});
it('auth/verify-totp pointer exports v1', () => {
  expect(p2.TOTP_STEP_SECONDS).toBe(v2.TOTP_STEP_SECONDS);
});
it('chats/build-model-turns pointer exports v1', () => {
  expect(p3.buildModelTurns).toBe(v3.buildModelTurns);
});
it('chats/map-citation pointer exports v1', () => {
  expect(p4.mapCitation).toBe(v4.mapCitation);
});
it('keys pointer exports v1', () => {
  expect(p5.DOC_SK_PREFIX).toBe(v5.DOC_SK_PREFIX);
});
it('usage/usage-day pointer exports v1', () => {
  expect(p6.usageDay).toBe(v6.usageDay);
});
