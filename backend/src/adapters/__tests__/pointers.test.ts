// Pointer tests (stack/versioning.md): each unit's index.ts exports the intended version.
import { expect, it } from 'vitest';
import * as p0 from '../anthropic-answer-model/index.js';
import * as v0 from '../anthropic-answer-model/v1.js';
import * as p1 from '../crypto-id-generator/index.js';
import * as v1 from '../crypto-id-generator/v1.js';
import * as p2 from '../dynamo/dynamo-auth-state-store/index.js';
import * as v2 from '../dynamo/dynamo-auth-state-store/v1.js';
import * as p3 from '../dynamo/dynamo-chat-store/index.js';
import * as v3 from '../dynamo/dynamo-chat-store/v1.js';
import * as p4 from '../dynamo/dynamo-doc-store/index.js';
import * as v4 from '../dynamo/dynamo-doc-store/v1.js';
import * as p5 from '../dynamo/dynamo-usage-meter/index.js';
import * as v5 from '../dynamo/dynamo-usage-meter/v1.js';
import * as p6 from '../s3-file-store/index.js';
import * as v6 from '../s3-file-store/v1.js';
import * as p7 from '../ssm/ssm-flag-reader/index.js';
import * as v7 from '../ssm/ssm-flag-reader/v1.js';
import * as p8 from '../ssm/ssm-secret-source/index.js';
import * as v8 from '../ssm/ssm-secret-source/v1.js';
import * as p9 from '../system-clock/index.js';
import * as v9 from '../system-clock/v1.js';

it('anthropic-answer-model pointer exports v1', () => {
  expect(p0.toParams).toBe(v0.toParams);
});
it('crypto-id-generator pointer exports v1', () => {
  expect(p1.cryptoIdGenerator).toBe(v1.cryptoIdGenerator);
});
it('dynamo/dynamo-auth-state-store pointer exports v1', () => {
  expect(p2.dynamoAuthStateStore).toBe(v2.dynamoAuthStateStore);
});
it('dynamo/dynamo-chat-store pointer exports v1', () => {
  expect(p3.dynamoChatStore).toBe(v3.dynamoChatStore);
});
it('dynamo/dynamo-doc-store pointer exports v1', () => {
  expect(p4.dynamoDocStore).toBe(v4.dynamoDocStore);
});
it('dynamo/dynamo-usage-meter pointer exports v1', () => {
  expect(p5.dynamoUsageMeter).toBe(v5.dynamoUsageMeter);
});
it('s3-file-store pointer exports v1', () => {
  expect(p6.s3FileStore).toBe(v6.s3FileStore);
});
it('ssm/ssm-flag-reader pointer exports v1', () => {
  expect(p7.ssmFlagReader).toBe(v7.ssmFlagReader);
});
it('ssm/ssm-secret-source pointer exports v1', () => {
  expect(p8.ssmSecretSource).toBe(v8.ssmSecretSource);
});
it('system-clock pointer exports v1', () => {
  expect(p9.systemClock).toBe(v9.systemClock);
});
