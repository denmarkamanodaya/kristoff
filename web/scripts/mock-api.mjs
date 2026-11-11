// Local smoke-test double of the Kristoff backend (docs/contract.md), node:http only, in memory.
// Serves web/dist on WEB_PORT and the HTTP API + ask stream + fake S3 on API_PORT (a second
// origin, so CORS and the Authorization preflight are exercised like in prod).
// TOTP: 123456 is valid, 111111 answers OTP_REPLAYED, 5 wrong codes in a row lock for 15 min.
// Test hook: `curl -X POST localhost:5301/_revoke` drops every token (next call -> UNAUTHENTICATED).
// Usage: npm run build && npm run mock   -> open http://localhost:5300
import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const WEB_PORT = Number(process.env.WEB_PORT ?? 5300);
const API_PORT = Number(process.env.API_PORT ?? 5301);
const API = `http://localhost:${API_PORT}`;
const WEB_ORIGIN = `http://localhost:${WEB_PORT}`;
const dist = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'dist');

const hex16 = () => randomBytes(8).toString('hex');
const ulid = () => (Date.now().toString(36).toUpperCase() + randomBytes(10).toString('hex').toUpperCase()).slice(0, 26).padEnd(26, '0');
const now = () => new Date().toISOString();
const zeroUsage = () => ({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0 });
const addUsage = (a, b) => { for (const k of Object.keys(a)) a[k] += b[k]; };

const db = {
  tokens: new Set(), failCount: 0, lockedUntil: 0,
  docs: new Map(), files: new Map(), chats: new Map(), messages: new Map(),
  usage: { today: zeroUsage(), total: zeroUsage() },
};
const models = [
  { id: 'claude-sonnet-5-5', label: 'Sonnet 5.5' },
  { id: 'claude-opus-5-5', label: 'Opus 5.5' },
  { id: 'claude-haiku-5-5', label: 'Haiku 5.5' },
];
const config = { model: models[0].id, models, maxAttachedDocs: 5, dailyQuestionCap: 50 };

// ---------- helpers ----------
const cors = {
  'Access-Control-Allow-Origin': WEB_ORIGIN,
  'Access-Control-Allow-Headers': 'Authorization, Content-Type',
  'Access-Control-Allow-Methods': 'GET, POST, PATCH, PUT, OPTIONS',
  'Access-Control-Expose-Headers': 'Retry-After',
};
function send(res, status, body, headers = {}) {
  res.writeHead(status, { ...cors, 'Content-Type': 'application/json', ...headers });
  res.end(JSON.stringify(body));
}
function fail(res, status, code, message, headers) {
  send(res, status, { error: { code, message, requestId: `req-${hex16()}` } }, headers);
}
async function readBody(req, limit = 40 * 1024 * 1024) {
  const chunks = []; let size = 0;
  for await (const c of req) { size += c.length; if (size > limit) throw new Error('too big'); chunks.push(c); }
  return Buffer.concat(chunks);
}
async function readJson(req) {
  const raw = await readBody(req, 64 * 1024);
  if (!raw.length) return {};
  return JSON.parse(raw.toString('utf8'));
}
const authed = (req) => db.tokens.has((req.headers.authorization ?? '').replace(/^Bearer /, ''));
const summary = (c) => ({ id: c.id, title: c.title, docIds: c.docIds, updatedAt: c.updatedAt });
const isId = (s) => /^[a-f0-9]{16}$/.test(s);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ---------- routes ----------
async function verify(req, res) {
  const { code } = await readJson(req);
  if (!/^\d{6}$/.test(code ?? '')) return fail(res, 400, 'VALIDATION_FAILED', 'code must be 6 digits');
  if (Date.now() < db.lockedUntil) {
    return fail(res, 429, 'OTP_LOCKED', 'Too many attempts', { 'Retry-After': String(Math.ceil((db.lockedUntil - Date.now()) / 1000)) });
  }
  if (code === '111111') return fail(res, 401, 'OTP_REPLAYED', 'Code already used');
  if (code !== '123456') {
    db.failCount += 1;
    if (db.failCount >= 5) { db.lockedUntil = Date.now() + 15 * 60_000; db.failCount = 0; }
    return fail(res, 401, 'OTP_INVALID', 'Wrong code');
  }
  db.failCount = 0;
  const token = randomBytes(24).toString('base64url');
  db.tokens.add(token);
  return send(res, 200, { token, expiresAt: new Date(Date.now() + 43200_000).toISOString() });
}

function page(list, cursor, size) {
  const start = cursor ? Number(Buffer.from(cursor, 'base64url').toString()) : 0;
  const items = list.slice(start, start + size);
  const next = start + size < list.length ? Buffer.from(String(start + size)).toString('base64url') : null;
  return { items, nextCursor: next };
}

async function createDocRoute(req, res) {
  const { fileName, size } = await readJson(req);
  if (typeof fileName !== 'string' || !fileName || fileName.length > 255 || !Number.isInteger(size) || size < 1 || size > 33554432) {
    return fail(res, 400, 'VALIDATION_FAILED', 'fileName and size are required');
  }
  const doc = { id: hex16(), title: fileName.replace(/\.pdf$/i, '').slice(0, 200), size, status: 'pending', createdAt: now() };
  db.docs.set(doc.id, doc);
  return send(res, 200, { doc, uploadUrl: `${API}/_s3/${doc.id}?sig=fake`, uploadHeaders: { 'Content-Type': 'application/pdf' } });
}

function confirmDocRoute(res, id) {
  const doc = db.docs.get(id);
  if (!doc) return fail(res, 404, 'DOC_NOT_FOUND', 'No such doc');
  const file = db.files.get(id);
  if (!file || file.length !== doc.size || file.subarray(0, 5).toString() !== '%PDF-') {
    return fail(res, 422, 'UPLOAD_INVALID', 'Not a PDF or size mismatch');
  }
  doc.status = 'ready';
  return send(res, 200, { doc });
}

async function chatPatch(req, res, chat) {
  const body = await readJson(req);
  if (body.title === undefined && body.docIds === undefined) return fail(res, 400, 'VALIDATION_FAILED', 'one field required');
  if (body.docIds !== undefined) {
    if (!Array.isArray(body.docIds) || body.docIds.length > config.maxAttachedDocs) return fail(res, 400, 'VALIDATION_FAILED', 'docIds');
    if (body.docIds.some((d) => db.docs.get(d)?.status !== 'ready')) return fail(res, 404, 'DOC_NOT_FOUND', 'unknown doc');
    chat.docIds = body.docIds;
  }
  if (typeof body.title === 'string') chat.title = body.title.slice(0, 120);
  chat.updatedAt = now();
  return send(res, 200, { chat });
}

async function askRoute(req, res, chat) {
  const { text, model = config.model } = await readJson(req);
  if (typeof text !== 'string' || !text.length || text.length > 8000) return fail(res, 400, 'VALIDATION_FAILED', 'text');
  if (typeof model !== 'string' || model.length > 64 || !models.some((m) => m.id === model)) {
    return fail(res, 400, 'VALIDATION_FAILED', 'model is not allowed');
  }
  db.lastAskBody = { text, model }; // test: GET /_last-ask shows what the client sent
  const msgs = db.messages.get(chat.id);
  if (!msgs.length) chat.title = text.slice(0, 60);
  msgs.push({ id: ulid(), role: 'user', text, citations: [], usage: null, model: null, stopReason: null, at: now() });
  res.writeHead(200, { ...cors, 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' });
  const ev = (event, data) => res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
  const docs = chat.docIds.map((d) => db.docs.get(d)).filter(Boolean);
  const d0 = docs[0] ?? null; const d1 = docs[1] ?? d0;
  const cite = (n, d, p, quote) => ({ n, docId: d?.id ?? null, docTitle: d?.title ?? 'General knowledge', page: d ? p : null, endPage: d ? p : null, quote });
  const parts = ['## Ruling\n\n', 'The Court held that the **contract is void** ', 'for lack of consent.'];
  let answer = '';
  const citations = [];
  for (const p of parts) { await sleep(150); answer += p; ev('text', { text: p }); }
  const c1 = cite(1, d0, 1, 'There was no meeting of the minds between the parties.');
  citations.push(c1); answer += ' [1]'; ev('citation', c1);
  await sleep(150);
  const more = '\n\nThe ratio rests on Article 1318 of the Civil Code.';
  answer += more; ev('text', { text: more });
  const c2 = cite(2, d1, 2, 'Article 1318. There is no contract unless the following requisites concur.');
  citations.push(c2); answer += ' [2]'; ev('citation', c2);
  const usage = { input: 1200, output: 64, cacheRead: 800, cacheWrite: 0 };
  addUsage(db.usage.today, usage); addUsage(db.usage.total, usage);
  const message = { id: ulid(), role: 'assistant', text: answer, citations, usage, model, stopReason: 'end_turn', at: now() };
  msgs.push(message);
  chat.updatedAt = now();
  ev('done', { message, usage });
  res.end();
}

async function api(req, res) {
  const url = new URL(req.url, API);
  const p = url.pathname; const m = req.method;
  if (m === 'OPTIONS') { res.writeHead(204, cors); return res.end(); }
  if (p.startsWith('/_s3/') && m === 'PUT') {
    const id = p.slice(5);
    if (req.headers['content-type'] !== 'application/pdf') { res.writeHead(403, cors); return res.end('<Error>SignatureDoesNotMatch</Error>'); }
    db.files.set(id, await readBody(req)); res.writeHead(200, cors); return res.end();
  }
  if (p.startsWith('/_file/') && m === 'GET') {
    const file = db.files.get(p.slice(7));
    if (!file) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'Content-Type': 'application/pdf' }); return res.end(file);
  }
  if (p === '/_last-ask' && m === 'GET') return send(res, 200, db.lastAskBody ?? null);
  if (p === '/_revoke' && m === 'POST') { db.tokens.clear(); res.writeHead(204, cors); return res.end(); } // test: force UNAUTHENTICATED
  if (p === '/v1/auth/verify' && m === 'POST') return verify(req, res);
  if (!authed(req)) return fail(res, 401, 'UNAUTHENTICATED', 'Missing or invalid token');
  if (p === '/v1/config' && m === 'GET') return send(res, 200, config);
  if (p === '/v1/usage' && m === 'GET') return send(res, 200, db.usage);
  if (p === '/v1/docs' && m === 'GET') return send(res, 200, page([...db.docs.values()].reverse(), url.searchParams.get('cursor'), 100));
  if (p === '/v1/docs' && m === 'POST') return createDocRoute(req, res);
  if (p === '/v1/chats' && m === 'GET') {
    const list = [...db.chats.values()].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).map(summary);
    return send(res, 200, page(list, url.searchParams.get('cursor'), 50));
  }
  if (p === '/v1/chats' && m === 'POST') {
    const { docIds = [] } = await readJson(req);
    if (!Array.isArray(docIds) || docIds.length > config.maxAttachedDocs) return fail(res, 400, 'VALIDATION_FAILED', 'docIds');
    if (docIds.some((d) => db.docs.get(d)?.status !== 'ready')) return fail(res, 404, 'DOC_NOT_FOUND', 'unknown doc');
    const chat = { id: hex16(), title: 'New chat', docIds, createdAt: now(), updatedAt: now() };
    db.chats.set(chat.id, chat); db.messages.set(chat.id, []);
    return send(res, 200, { chat });
  }
  let r = /^\/v1\/docs\/([^/]+)(\/confirm|\/link)?$/.exec(p);
  if (r) {
    const [, id, sub] = r;
    if (!isId(id) || !db.docs.has(id)) return fail(res, 404, 'DOC_NOT_FOUND', 'No such doc');
    if (sub === '/confirm' && m === 'POST') return confirmDocRoute(res, id);
    if (sub === '/link' && m === 'GET') return send(res, 200, { url: `${API}/_file/${id}?sig=fake`, expiresAt: new Date(Date.now() + 300_000).toISOString() });
    if (!sub && m === 'PATCH') {
      const { title } = await readJson(req);
      if (typeof title !== 'string' || !title || title.length > 200) return fail(res, 400, 'VALIDATION_FAILED', 'title');
      const doc = db.docs.get(id); doc.title = title; return send(res, 200, { doc });
    }
  }
  r = /^\/v1\/chats\/([^/]+)(\/ask)?$/.exec(p);
  if (r) {
    const [, id, sub] = r;
    const chat = db.chats.get(id);
    if (!chat) return fail(res, 404, 'CHAT_NOT_FOUND', 'No such chat');
    if (sub === '/ask' && m === 'POST') return askRoute(req, res, chat);
    if (!sub && m === 'GET') return send(res, 200, { chat, messages: db.messages.get(id) });
    if (!sub && m === 'PATCH') return chatPatch(req, res, chat);
  }
  return fail(res, 404, 'VALIDATION_FAILED', `No route ${m} ${p}`);
}

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };
async function web(req, res) {
  const p = new URL(req.url, WEB_ORIGIN).pathname;
  if (p === '/config.json') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify({ apiBaseUrl: API, streamUrl: API }));
  }
  const file = normalize(join(dist, p === '/' ? 'index.html' : p));
  if (!file.startsWith(dist)) { res.writeHead(403); return res.end(); }
  try {
    const body = await readFile(file);
    res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream' });
    return res.end(body);
  } catch {
    res.writeHead(404); return res.end('not found');
  }
}

const wrap = (fn) => (req, res) => fn(req, res).catch((err) => {
  console.error(err);
  if (!res.headersSent) fail(res, 500, 'INTERNAL', 'mock error');
  else res.end();
});
createServer(wrap(api)).listen(API_PORT, () => console.log(`mock api   ${API}`));
createServer(wrap(web)).listen(WEB_PORT, () => console.log(`mock web   ${WEB_ORIGIN}  (TOTP code 123456)`));
