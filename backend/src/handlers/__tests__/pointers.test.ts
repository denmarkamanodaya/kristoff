// Pointer tests (stack/versioning.md): each unit's index.ts exports the intended version.
import { expect, it } from 'vitest';
import * as p0 from '../ask-stream/index.js';
import * as v0 from '../ask-stream/v1.js';
import * as p1 from '../auth-verify/index.js';
import * as v1 from '../auth-verify/v1.js';
import * as p2 from '../authorizer/index.js';
import * as v2 from '../authorizer/v1.js';
import * as p3 from '../chats-create/index.js';
import * as v3 from '../chats-create/v1.js';
import * as p4 from '../chats-delete/index.js';
import * as v4 from '../chats-delete/v1.js';
import * as p5 from '../chats-get/index.js';
import * as v5 from '../chats-get/v1.js';
import * as p6 from '../chats-list/index.js';
import * as v6 from '../chats-list/v1.js';
import * as p7 from '../chats-update/index.js';
import * as v7 from '../chats-update/v1.js';
import * as p8 from '../config-get/index.js';
import * as v8 from '../config-get/v1.js';
import * as p9 from '../docs-confirm/index.js';
import * as v9 from '../docs-confirm/v1.js';
import * as p10 from '../docs-create/index.js';
import * as v10 from '../docs-create/v1.js';
import * as p11 from '../docs-link/index.js';
import * as v11 from '../docs-link/v1.js';
import * as p12 from '../docs-list/index.js';
import * as v12 from '../docs-list/v1.js';
import * as p13 from '../docs-rename/index.js';
import * as v13 from '../docs-rename/v1.js';
import * as p14 from '../usage-get/index.js';
import * as v14 from '../usage-get/v1.js';

it('ask-stream pointer exports v1', () => {
  expect(p0.createAskStreamHandler).toBe(v0.createAskStreamHandler);
});
it('auth-verify pointer exports v1', () => {
  expect(p1.createAuthVerifyHandler).toBe(v1.createAuthVerifyHandler);
});
it('authorizer pointer exports v1', () => {
  expect(p2.createAuthorizer).toBe(v2.createAuthorizer);
});
it('chats-create pointer exports v1', () => {
  expect(p3.createChatsCreateHandler).toBe(v3.createChatsCreateHandler);
});
it('chats-delete pointer exports v1', () => {
  expect(p4.createChatsDeleteHandler).toBe(v4.createChatsDeleteHandler);
});
it('chats-get pointer exports v1', () => {
  expect(p5.createChatsGetHandler).toBe(v5.createChatsGetHandler);
});
it('chats-list pointer exports v1', () => {
  expect(p6.createChatsListHandler).toBe(v6.createChatsListHandler);
});
it('chats-update pointer exports v1', () => {
  expect(p7.createChatsUpdateHandler).toBe(v7.createChatsUpdateHandler);
});
it('config-get pointer exports v1', () => {
  expect(p8.createConfigGetHandler).toBe(v8.createConfigGetHandler);
});
it('docs-confirm pointer exports v1', () => {
  expect(p9.createDocsConfirmHandler).toBe(v9.createDocsConfirmHandler);
});
it('docs-create pointer exports v1', () => {
  expect(p10.createDocsCreateHandler).toBe(v10.createDocsCreateHandler);
});
it('docs-link pointer exports v1', () => {
  expect(p11.createDocsLinkHandler).toBe(v11.createDocsLinkHandler);
});
it('docs-list pointer exports v1', () => {
  expect(p12.createDocsListHandler).toBe(v12.createDocsListHandler);
});
it('docs-rename pointer exports v1', () => {
  expect(p13.createDocsRenameHandler).toBe(v13.createDocsRenameHandler);
});
it('usage-get pointer exports v1', () => {
  expect(p14.createUsageGetHandler).toBe(v14.createUsageGetHandler);
});
