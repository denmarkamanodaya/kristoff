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
import * as p5 from '../docs/attachment-id/index.js';
import * as v5 from '../docs/attachment-id/v1.js';
import * as p6 from '../docs/attachment-target/index.js';
import * as v6 from '../docs/attachment-target/v1.js';
import * as p7 from '../docs/plan-parts/index.js';
import * as v7 from '../docs/plan-parts/v1.js';
import * as p8 from '../keys/index.js';
import * as v8 from '../keys/v1.js';
import * as p9 from '../projects/project-name/index.js';
import * as v9 from '../projects/project-name/v1.js';
import * as p10 from '../usage/sum-usage/index.js';
import * as v10 from '../usage/sum-usage/v1.js';
import * as p11 from '../usage/usage-day/index.js';
import * as v11 from '../usage/usage-day/v1.js';

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
it('docs/attachment-id pointer exports v1', () => {
  expect(p5.parseAttachmentId).toBe(v5.parseAttachmentId);
});
it('docs/attachment-target pointer exports v1', () => {
  expect(p6.attachmentTarget).toBe(v6.attachmentTarget);
});
it('docs/plan-parts pointer exports v1', () => {
  expect(p7.needsParts).toBe(v7.needsParts);
});
it('keys pointer exports v1', () => {
  expect(p8.PROJECT_SK_PREFIX).toBe(v8.PROJECT_SK_PREFIX);
});
it('projects/project-name pointer exports v1', () => {
  expect(p9.projectName).toBe(v9.projectName);
});
it('usage/sum-usage pointer exports v1', () => {
  expect(p10.sumUsage).toBe(v10.sumUsage);
});
it('usage/usage-day pointer exports v1', () => {
  expect(p11.usageDay).toBe(v11.usageDay);
});
