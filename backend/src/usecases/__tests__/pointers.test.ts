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
import * as p4 from '../create-project/index.js';
import * as v4 from '../create-project/v1.js';
import * as p5 from '../delete-chat/index.js';
import * as v5 from '../delete-chat/v1.js';
import * as p6 from '../delete-doc/index.js';
import * as v6 from '../delete-doc/v1.js';
import * as p7 from '../delete-project/index.js';
import * as v7 from '../delete-project/v1.js';
import * as p8 from '../each-owner-chat/index.js';
import * as v8 from '../each-owner-chat/v1.js';
import * as p9 from '../get-chat/index.js';
import * as v9 from '../get-chat/v1.js';
import * as p10 from '../get-config/index.js';
import * as v10 from '../get-config/v1.js';
import * as p11 from '../get-doc-link/index.js';
import * as v11 from '../get-doc-link/v1.js';
import * as p12 from '../get-usage/index.js';
import * as v12 from '../get-usage/v1.js';
import * as p13 from '../list-chats/index.js';
import * as v13 from '../list-chats/v1.js';
import * as p14 from '../list-docs/index.js';
import * as v14 from '../list-docs/v1.js';
import * as p15 from '../list-projects/index.js';
import * as v15 from '../list-projects/v1.js';
import * as p16 from '../load-attachments/index.js';
import * as v16 from '../load-attachments/v1.js';
import * as p17 from '../load-owned-chat/index.js';
import * as v17 from '../load-owned-chat/v1.js';
import * as p18 from '../rename-doc/index.js';
import * as v18 from '../rename-doc/v1.js';
import * as p19 from '../rename-project/index.js';
import * as v19 from '../rename-project/v1.js';
import * as p20 from '../resolve-attached-docs/index.js';
import * as v20 from '../resolve-attached-docs/v1.js';
import * as p21 from '../split-doc/index.js';
import * as v21 from '../split-doc/v1.js';
import * as p22 from '../update-chat/index.js';
import * as v22 from '../update-chat/v1.js';
import * as p23 from '../verify-otp/index.js';
import * as v23 from '../verify-otp/v1.js';
import * as p24 from '../verify-session/index.js';
import * as v24 from '../verify-session/v1.js';

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
it('create-project pointer exports v1', () => {
  expect(p4.createProject).toBe(v4.createProject);
});
it('delete-chat pointer exports v1', () => {
  expect(p5.deleteChat).toBe(v5.deleteChat);
});
it('delete-doc pointer exports v1', () => {
  expect(p6.deleteDoc).toBe(v6.deleteDoc);
});
it('delete-project pointer exports v1', () => {
  expect(p7.deleteProject).toBe(v7.deleteProject);
});
it('each-owner-chat pointer exports v1', () => {
  expect(p8.eachOwnerChat).toBe(v8.eachOwnerChat);
});
it('get-chat pointer exports v1', () => {
  expect(p9.getChat).toBe(v9.getChat);
});
it('get-config pointer exports v1', () => {
  expect(p10.getConfig).toBe(v10.getConfig);
});
it('get-doc-link pointer exports v1', () => {
  expect(p11.getDocLink).toBe(v11.getDocLink);
});
it('get-usage pointer exports v1', () => {
  expect(p12.getUsage).toBe(v12.getUsage);
});
it('list-chats pointer exports v1', () => {
  expect(p13.listChats).toBe(v13.listChats);
});
it('list-docs pointer exports v1', () => {
  expect(p14.listDocs).toBe(v14.listDocs);
});
it('list-projects pointer exports v1', () => {
  expect(p15.listProjects).toBe(v15.listProjects);
});
it('load-attachments pointer exports v1', () => {
  expect(p16.loadAttachments).toBe(v16.loadAttachments);
});
it('load-owned-chat pointer exports v1', () => {
  expect(p17.loadOwnedChat).toBe(v17.loadOwnedChat);
});
it('rename-doc pointer exports v1', () => {
  expect(p18.renameDoc).toBe(v18.renameDoc);
});
it('rename-project pointer exports v1', () => {
  expect(p19.renameProject).toBe(v19.renameProject);
});
it('resolve-attached-docs pointer exports v1', () => {
  expect(p20.resolveAttachedDocs).toBe(v20.resolveAttachedDocs);
});
it('split-doc pointer exports v1', () => {
  expect(p21.splitDoc).toBe(v21.splitDoc);
});
it('update-chat pointer exports v1', () => {
  expect(p22.updateChat).toBe(v22.updateChat);
});
it('verify-otp pointer exports v1', () => {
  expect(p23.verifyOtp).toBe(v23.verifyOtp);
});
it('verify-session pointer exports v1', () => {
  expect(p24.verifySession).toBe(v24.verifySession);
});
