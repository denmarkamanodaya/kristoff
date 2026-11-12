// Wire tests for services/api-desktop.js against a fake `window.__TAURI__`: each export must send
// the exact command name and argument object from docs/contract.md "Desktop IPC", uploads must
// chunk at 8 MiB with the right offsets, and Channel events must map to the api.js callbacks.
// Run: `npm test` (node --test, no dependencies).
import { test, beforeEach, after } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { exportNames } from '../scripts/check.mjs';

const calls = [];
let reply = () => ({});
let lastChannel = null;

class FakeChannel {
  constructor() {
    this.onmessage = null;
    lastChannel = this;
  }
}

globalThis.window = {
  __TAURI__: {
    core: {
      Channel: FakeChannel,
      invoke: async (cmd, args, opts) => {
        calls.push(opts === undefined ? { cmd, args } : { cmd, args, opts });
        return reply(cmd, args, opts);
      },
    },
  },
};

const api = await import('../services/api-desktop.js');
const session = await import('../services/session.js');

const unlock = () => session.setSession({ token: 't', expiresAt: new Date(Date.now() + 3_600_000).toISOString() });

beforeEach(() => {
  calls.length = 0;
  reply = () => ({});
  lastChannel = null;
  unlock();
});
after(() => session.expire()); // clears the expiry timer so the process exits

test('exports exactly the names services/api.js exports', () => {
  const read = (f) => exportNames(readFileSync(new URL(`../services/${f}`, import.meta.url), 'utf8'));
  assert.deepEqual(read('api-desktop.js'), read('api.js'));
  assert.deepEqual(exportNames('export { a, b as c };\nexport async function d() {}'), ['a', 'c', 'd']);
});

test('every request function sends its command name and camelCase args, null for absent optionals', async () => {
  const cases = [
    [() => api.verifyCode('123456'), 'verify_otp', { code: '123456' }],
    [() => api.getConfig(), 'get_config', {}],
    [() => api.listDocs(), 'list_docs', { cursor: null }],
    [() => api.listDocs('c1'), 'list_docs', { cursor: 'c1' }],
    [() => api.createDoc('a.pdf', 42), 'create_doc', { fileName: 'a.pdf', size: 42 }],
    [() => api.confirmDoc('d1'), 'confirm_doc', { docId: 'd1' }],
    [() => api.renameDoc('d1', 'T'), 'rename_doc', { docId: 'd1', title: 'T' }],
    [() => api.getDocLink('d1'), 'get_doc_link', { docId: 'd1', part: null }],
    [() => api.getDocLink('d1', 2), 'get_doc_link', { docId: 'd1', part: 2 }],
    [() => api.deleteDoc('d1'), 'delete_doc', { docId: 'd1' }],
    [() => api.listProjects(), 'list_projects', {}],
    [() => api.createProject('P'), 'create_project', { name: 'P' }],
    [() => api.renameProject('p1', 'Q'), 'rename_project', { projectId: 'p1', name: 'Q' }],
    [() => api.deleteProject('p1'), 'delete_project', { projectId: 'p1' }],
    [() => api.listChats(), 'list_chats', { cursor: null }],
    [() => api.listChats('c2'), 'list_chats', { cursor: 'c2' }],
    [() => api.createChat(['d1']), 'create_chat', { docIds: ['d1'] }],
    [() => api.getChat('c1'), 'get_chat', { chatId: 'c1' }],
    [() => api.updateChat('c1', { pinned: true }), 'update_chat', { chatId: 'c1', patch: { pinned: true } }],
    [() => api.deleteChat('c1'), 'delete_chat', { chatId: 'c1' }],
    [() => api.getUsage(), 'get_usage', {}],
    [() => api.openDoc('d1'), 'open_doc', { docId: 'd1', part: null, page: null }],
    [() => api.openDoc('d1', 2, 37), 'open_doc', { docId: 'd1', part: 2, page: 37 }],
    [() => api.openExternal('https://example.org/x'), 'open_external', { url: 'https://example.org/x' }],
  ];
  for (const [run, cmd, args] of cases) {
    calls.length = 0;
    await run();
    assert.deepEqual(calls, [{ cmd, args }], cmd);
  }
});

test('resolves with the command value unchanged', async () => {
  reply = () => ({ token: 'abc', expiresAt: '2026-11-12T00:00:00.000Z' });
  assert.deepEqual(await api.verifyCode('123456'), { token: 'abc', expiresAt: '2026-11-12T00:00:00.000Z' });
});

test('maps a rejected ErrorDto to ApiError with code, message and retryAfter', async () => {
  reply = () => Promise.reject({ code: 'OTP_LOCKED', message: 'Wait', retryAfter: 30 });
  const err = await api.verifyCode('123456').catch((e) => e);
  assert.ok(err instanceof api.ApiError);
  assert.equal(err.code, 'OTP_LOCKED');
  assert.equal(err.message, 'Wait');
  assert.equal(err.retryAfter, 30);
  assert.equal(err.requestId, null);
});

test('maps a string rejection (Tauri-level failure) to INTERNAL', async () => {
  reply = () => Promise.reject('command list_docs not found');
  const err = await api.listDocs().catch((e) => e);
  assert.equal(err.code, 'INTERNAL');
  assert.equal(err.message, 'command list_docs not found');
});

test('UNAUTHENTICATED from the shell clears the session', async () => {
  reply = () => Promise.reject({ code: 'UNAUTHENTICATED', message: 'Locked', retryAfter: null });
  const err = await api.getUsage().catch((e) => e);
  assert.equal(err.code, 'UNAUTHENTICATED');
  assert.equal(session.getToken(), null);
});

test('a locked session fails UNAUTHENTICATED without calling the shell, except verify_otp', async () => {
  session.expire();
  const err = await api.getUsage().catch((e) => e);
  assert.equal(err.code, 'UNAUTHENTICATED');
  assert.equal(calls.length, 0);
  await api.verifyCode('123456');
  assert.equal(calls[0].cmd, 'verify_otp');
});

test('uploadFile sends a 20 MiB file as 8 MiB raw chunks with doc id and offset headers', async () => {
  const MiB = 1024 * 1024;
  const bytes = new Uint8Array(20 * MiB);
  bytes[8 * MiB] = 7; // first byte of chunk 2, to prove the slice boundaries
  const file = new File([bytes], 'a.pdf', { type: 'application/pdf' });
  const progress = [];
  const result = await api.uploadFile('kristoff-upload://01DOC', { 'Content-Type': 'application/pdf' }, file, (f) => progress.push(f));
  assert.equal(result, undefined);
  assert.deepEqual(
    calls.map((c) => [c.cmd, c.args.byteLength, c.opts]),
    [
      ['upload_chunk', 8 * MiB, { headers: { 'x-doc-id': '01DOC', 'x-offset': '0' } }],
      ['upload_chunk', 8 * MiB, { headers: { 'x-doc-id': '01DOC', 'x-offset': String(8 * MiB) } }],
      ['upload_chunk', 4 * MiB, { headers: { 'x-doc-id': '01DOC', 'x-offset': String(16 * MiB) } }],
    ],
  );
  assert.ok(calls.every((c) => c.args instanceof ArrayBuffer));
  assert.equal(new Uint8Array(calls[1].args)[0], 7);
  assert.deepEqual(progress, [0.4, 0.8, 1]);
});

test('uploadFile rejects a non kristoff-upload URL and stops on the first failed chunk', async () => {
  const file = new File([new Uint8Array(10)], 'a.pdf');
  assert.equal((await api.uploadFile('https://s3/x', {}, file).catch((e) => e)).code, 'UPLOAD_FAILED');
  assert.equal(calls.length, 0);
  reply = () => Promise.reject({ code: 'UPLOAD_INVALID', message: 'Size mismatch', retryAfter: null });
  const big = new File([new Uint8Array(9 * 1024 * 1024)], 'a.pdf');
  const err = await api.uploadFile('kristoff-upload://d1', {}, big).catch((e) => e);
  assert.equal(err.code, 'UPLOAD_INVALID');
  assert.equal(calls.length, 1);
});

test('parseSse is a no-op stub on desktop (D-10)', () => {
  assert.deepEqual(api.parseSse('event: text\ndata: {}\n\n'), { events: [], rest: '' });
});

/** Starts askStream with a reply that never settles unless `settle` is called. */
function startAsk(handlers) {
  let settle;
  reply = (cmd) => (cmd === 'ask' ? new Promise((res, rej) => { settle = { res, rej }; }) : undefined);
  const p = api.askStream('c1', 'Q?', handlers);
  return { p, settle: () => settle };
}

test('askStream sends ask with a Channel and maps every event to its callback', async () => {
  const seen = [];
  const { p } = startAsk({
    model: 'claude-sonnet-5-5',
    onText: (t) => seen.push(['text', t]),
    onCitation: (c) => seen.push(['citation', c]),
    onFollowUps: (l) => seen.push(['followups', l]),
  });
  await Promise.resolve();
  assert.deepEqual(calls, [{ cmd: 'ask', args: { chatId: 'c1', text: 'Q?', model: 'claude-sonnet-5-5', onEvent: lastChannel } }]);
  assert.ok(lastChannel instanceof FakeChannel);
  const cite = { n: 1, docId: 'd1', docTitle: 'D', page: 2, endPage: 2, quote: 'q', kind: 'doc', sourceUrl: null };
  lastChannel.onmessage({ event: 'text', data: { text: 'Hi' } });
  lastChannel.onmessage({ event: 'citation', data: cite });
  lastChannel.onmessage({ event: 'followups', data: { followUps: ['Why?'] } });
  lastChannel.onmessage({ event: 'done', data: { message: { id: 'm' }, usage: { input: 1 } } });
  assert.deepEqual(await p, { message: { id: 'm' }, usage: { input: 1 } });
  assert.deepEqual(seen, [['text', 'Hi'], ['citation', cite], ['followups', ['Why?']]]);
});

test('askStream sends model null when no model is chosen', async () => {
  const { p } = startAsk({});
  await Promise.resolve();
  assert.equal(calls[0].args.model, null);
  lastChannel.onmessage({ event: 'done', data: { message: {}, usage: {} } });
  await p;
});

test('askStream rejects with ApiError on an error event', async () => {
  const { p } = startAsk({});
  await Promise.resolve();
  lastChannel.onmessage({ event: 'error', data: { error: { code: 'DAILY_CAP_REACHED', message: 'Cap', requestId: 'r1' } } });
  const err = await p.catch((e) => e);
  assert.ok(err instanceof api.ApiError);
  assert.deepEqual([err.code, err.message, err.requestId], ['DAILY_CAP_REACHED', 'Cap', 'r1']);
});

test('askStream rejects with the mapped ApiError when ask itself is rejected', async () => {
  reply = () => Promise.reject({ code: 'AI_DISABLED', message: 'Off', retryAfter: 60 });
  const err = await api.askStream('c1', 'Q?', {}).catch((e) => e);
  assert.deepEqual([err.code, err.retryAfter], ['AI_DISABLED', 60]);
});

test('askStream rejects UPSTREAM_UNAVAILABLE when ask returns without done', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const { p, settle } = startAsk({});
  await Promise.resolve();
  settle().res(null);
  await new Promise((r) => setImmediate(r));
  t.mock.timers.tick(2_000);
  assert.equal((await p.catch((e) => e)).code, 'UPSTREAM_UNAVAILABLE');
});

test('askStream still resolves when done arrives just after ask returns', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const { p, settle } = startAsk({});
  await Promise.resolve();
  settle().res(null);
  await new Promise((r) => setImmediate(r));
  t.mock.timers.tick(1_999); // inside the 2 s grace window
  lastChannel.onmessage({ event: 'done', data: { message: { id: 'late' }, usage: {} } });
  assert.deepEqual(await p, { message: { id: 'late' }, usage: {} });
});

test('askStream abort sends cancel_ask, rejects with AbortError and ignores later events', async () => {
  const ctrl = new AbortController();
  const seen = [];
  const { p } = startAsk({ signal: ctrl.signal, onText: (t) => seen.push(t) });
  await Promise.resolve();
  ctrl.abort();
  const err = await p.catch((e) => e);
  assert.equal(err.name, 'AbortError');
  assert.deepEqual(calls.map((c) => [c.cmd, c.args.chatId]), [['ask', 'c1'], ['cancel_ask', 'c1']]);
  lastChannel.onmessage({ event: 'text', data: { text: 'late' } });
  assert.deepEqual(seen, []);
});

test('askStream with an already aborted signal never calls ask', async () => {
  const ctrl = new AbortController();
  ctrl.abort();
  const err = await api.askStream('c1', 'Q?', { signal: ctrl.signal }).catch((e) => e);
  assert.equal(err.name, 'AbortError');
  assert.equal(calls.length, 0);
});
