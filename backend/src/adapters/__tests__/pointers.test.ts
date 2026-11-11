// Pointer tests (stack/versioning.md): each unit's index.ts exports the intended version.
import { expect, it } from 'vitest';
import * as p0 from '../anthropic-answer-model/index.js';
import * as v0 from '../anthropic-answer-model/v1.js';
import * as p1 from '../anthropic-follow-up-suggester/index.js';
import * as v1 from '../anthropic-follow-up-suggester/v1.js';
import * as p2 from '../crypto-id-generator/index.js';
import * as v2 from '../crypto-id-generator/v1.js';
import * as p3 from '../dynamo/dynamo-auth-state-store/index.js';
import * as v3 from '../dynamo/dynamo-auth-state-store/v1.js';
import * as p4 from '../dynamo/dynamo-chat-store/index.js';
import * as v4 from '../dynamo/dynamo-chat-store/v1.js';
import * as p5 from '../dynamo/dynamo-doc-store/index.js';
import * as v5 from '../dynamo/dynamo-doc-store/v1.js';
import * as p6 from '../dynamo/dynamo-project-store/index.js';
import * as v6 from '../dynamo/dynamo-project-store/v1.js';
import * as p7 from '../dynamo/dynamo-usage-meter/index.js';
import * as v7 from '../dynamo/dynamo-usage-meter/v1.js';
import * as p8 from '../lambda-doc-split-trigger/index.js';
import * as v8 from '../lambda-doc-split-trigger/v1.js';
import * as p9 from '../pdf-lib-reader/index.js';
import * as v9 from '../pdf-lib-reader/v1.js';
import * as p10 from '../s3-file-store/index.js';
import * as v10 from '../s3-file-store/v1.js';
import * as p11 from '../ssm/ssm-flag-reader/index.js';
import * as v11 from '../ssm/ssm-flag-reader/v1.js';
import * as p12 from '../ssm/ssm-secret-source/index.js';
import * as v12 from '../ssm/ssm-secret-source/v1.js';
import * as p13 from '../system-clock/index.js';
import * as v13 from '../system-clock/v1.js';

it('anthropic-answer-model pointer exports v1', () => {
  expect(p0.toParams).toBe(v0.toParams);
});
it('anthropic-follow-up-suggester pointer exports v1', () => {
  expect(p1.toFollowUpParams).toBe(v1.toFollowUpParams);
});
it('crypto-id-generator pointer exports v1', () => {
  expect(p2.cryptoIdGenerator).toBe(v2.cryptoIdGenerator);
});
it('dynamo/dynamo-auth-state-store pointer exports v1', () => {
  expect(p3.dynamoAuthStateStore).toBe(v3.dynamoAuthStateStore);
});
it('dynamo/dynamo-chat-store pointer exports v1', () => {
  expect(p4.dynamoChatStore).toBe(v4.dynamoChatStore);
});
it('dynamo/dynamo-doc-store pointer exports v1', () => {
  expect(p5.dynamoDocStore).toBe(v5.dynamoDocStore);
});
it('dynamo/dynamo-project-store pointer exports v1', () => {
  expect(p6.dynamoProjectStore).toBe(v6.dynamoProjectStore);
});
it('dynamo/dynamo-usage-meter pointer exports v1', () => {
  expect(p7.dynamoUsageMeter).toBe(v7.dynamoUsageMeter);
});
it('lambda-doc-split-trigger pointer exports v1', () => {
  expect(p8.lambdaDocSplitTrigger).toBe(v8.lambdaDocSplitTrigger);
});
it('pdf-lib-reader pointer exports v1', () => {
  expect(p9.pdfLibReader).toBe(v9.pdfLibReader);
});
it('s3-file-store pointer exports v1', () => {
  expect(p10.s3FileStore).toBe(v10.s3FileStore);
});
it('ssm/ssm-flag-reader pointer exports v1', () => {
  expect(p11.ssmFlagReader).toBe(v11.ssmFlagReader);
});
it('ssm/ssm-secret-source pointer exports v1', () => {
  expect(p12.ssmSecretSource).toBe(v12.ssmSecretSource);
});
it('system-clock pointer exports v1', () => {
  expect(p13.systemClock).toBe(v13.systemClock);
});
