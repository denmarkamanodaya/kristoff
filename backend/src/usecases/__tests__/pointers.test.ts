// Pointer tests (stack/versioning.md): each unit's index.ts exports the intended version.
import { expect, it } from 'vitest';
import * as p0 from '../ask-question/index.js';
import * as v0 from '../ask-question/v1.js';
import * as p1 from '../confirm-doc/index.js';
import * as v1 from '../confirm-doc/v1.js';
import * as p2 from '../create-chat/index.js';
import * as v2 from '../create-chat/v1.js';
import * as p3 from '../create-doc-upload/index.js';
import * as v3 from '../create-doc-upload/v1.js';
import * as p4 from '../get-chat/index.js';
import * as v4 from '../get-chat/v1.js';
import * as p5 from '../get-config/index.js';
import * as v5 from '../get-config/v1.js';
import * as p6 from '../get-doc-link/index.js';
import * as v6 from '../get-doc-link/v1.js';
import * as p7 from '../get-usage/index.js';
import * as v7 from '../get-usage/v1.js';
import * as p8 from '../list-chats/index.js';
import * as v8 from '../list-chats/v1.js';
import * as p9 from '../list-docs/index.js';
import * as v9 from '../list-docs/v1.js';
import * as p10 from '../rename-doc/index.js';
import * as v10 from '../rename-doc/v1.js';
import * as p11 from '../resolve-attached-docs/index.js';
import * as v11 from '../resolve-attached-docs/v1.js';
import * as p12 from '../update-chat/index.js';
import * as v12 from '../update-chat/v1.js';
import * as p13 from '../verify-otp/index.js';
import * as v13 from '../verify-otp/v1.js';
import * as p14 from '../verify-session/index.js';
import * as v14 from '../verify-session/v1.js';

it('ask-question pointer exports v1', () => {
  expect(p0.askQuestion).toBe(v0.askQuestion);
});
it('confirm-doc pointer exports v1', () => {
  expect(p1.confirmDoc).toBe(v1.confirmDoc);
});
it('create-chat pointer exports v1', () => {
  expect(p2.createChat).toBe(v2.createChat);
});
it('create-doc-upload pointer exports v1', () => {
  expect(p3.createDocUpload).toBe(v3.createDocUpload);
});
it('get-chat pointer exports v1', () => {
  expect(p4.getChat).toBe(v4.getChat);
});
it('get-config pointer exports v1', () => {
  expect(p5.getConfig).toBe(v5.getConfig);
});
it('get-doc-link pointer exports v1', () => {
  expect(p6.getDocLink).toBe(v6.getDocLink);
});
it('get-usage pointer exports v1', () => {
  expect(p7.getUsage).toBe(v7.getUsage);
});
it('list-chats pointer exports v1', () => {
  expect(p8.listChats).toBe(v8.listChats);
});
it('list-docs pointer exports v1', () => {
  expect(p9.listDocs).toBe(v9.listDocs);
});
it('rename-doc pointer exports v1', () => {
  expect(p10.renameDoc).toBe(v10.renameDoc);
});
it('resolve-attached-docs pointer exports v1', () => {
  expect(p11.resolveAttachedDocs).toBe(v11.resolveAttachedDocs);
});
it('update-chat pointer exports v1', () => {
  expect(p12.updateChat).toBe(v12.updateChat);
});
it('verify-otp pointer exports v1', () => {
  expect(p13.verifyOtp).toBe(v13.verifyOtp);
});
it('verify-session pointer exports v1', () => {
  expect(p14.verifySession).toBe(v14.verifySession);
});
