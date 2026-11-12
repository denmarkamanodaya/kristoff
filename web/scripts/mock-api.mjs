// Local smoke-test double of the Kristoff backend (docs/contract.md, including "v2"), node:http only,
// in memory. Serves the built site on WEB_PORT and the HTTP API + ask stream + fake S3 on API_PORT
// (a second origin, so CORS and the Authorization preflight are exercised like in prod).
// TOTP: 123456 is valid, 111111 answers OTP_REPLAYED, 5 wrong codes in a row lock for 15 min.
// MOCK_ANY_CODE=1: any 6-digit code unlocks (no replay or lockout), for a local preview.
// Large PDFs: a file over 30 MB, or whose name contains "large", turns "processing" on confirm and
// "ready" 3 s later as a 600-page doc with 6 parts; a name containing "fail" turns "failed" instead.
// Test hooks: POST /_revoke drops every token; GET /_last-ask returns the last ask body.
// MOCK_STATUTES=1: config gains `features: { statutes: true, lawCount }` (desktop-style), so the composer shows the
// Statutes toggle and PATCH accepts `statutes`. Every answer emits one `search` event and one law citation.
// Env: WEB_PORT (5300), API_PORT (5301), DIST_DIR (dist).
// Usage: npm run build && npm run mock   -> open http://localhost:5300
import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const WEB_PORT = Number(process.env.WEB_PORT ?? 5300);
const API_PORT = Number(process.env.API_PORT ?? 5301);
const ANY_CODE = process.env.MOCK_ANY_CODE === '1';
const API = `http://localhost:${API_PORT}`;
const WEB_ORIGIN = `http://localhost:${WEB_PORT}`;
const dist = resolve(dirname(fileURLToPath(import.meta.url)), '..', process.env.DIST_DIR ?? 'dist');

const MAX_UPLOAD = 209_715_200; // contract v2: 200 MB
const SPLIT_BYTES = 30 * 1024 * 1024;
const PROJECT_CAP = 50;
const ATTACH_ID = /^[a-f0-9]{16}(:p[0-9]{1,3})?$/;

const hex16 = () => randomBytes(8).toString('hex');
const ulid = () => (Date.now().toString(36).toUpperCase() + randomBytes(10).toString('hex').toUpperCase()).slice(0, 26).padEnd(26, '0');
const now = () => new Date().toISOString();
const zeroUsage = () => ({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0 });
const addUsage = (a, b) => { for (const k of Object.keys(a)) a[k] += b[k]; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const isId = (s) => /^[a-f0-9]{16}$/.test(s);

const db = {
  tokens: new Set(), failCount: 0, lockedUntil: 0, lastAskBody: null,
  docs: new Map(), files: new Map(), chats: new Map(), messages: new Map(), projects: new Map(),
  usage: { today: zeroUsage(), total: zeroUsage() },
};
const models = [
  { id: 'claude-sonnet-5-5', label: 'Sonnet 5.5' },
  { id: 'claude-opus-5-5', label: 'Opus 5.5' },
  { id: 'claude-haiku-5-5', label: 'Haiku 5.5' },
];
const config = { model: models[0].id, models, maxAttachedDocs: 5, dailyQuestionCap: 50 };
if (process.env.MOCK_STATUTES === '1') config.features = { statutes: true, lawCount: 10870 }; // sample count

// ---------- helpers ----------
const cors = {
  'Access-Control-Allow-Origin': WEB_ORIGIN,
  'Access-Control-Allow-Headers': 'Authorization, Content-Type',
  'Access-Control-Allow-Methods': 'GET, POST, PATCH, PUT, DELETE, OPTIONS',
  'Access-Control-Expose-Headers': 'Retry-After',
};
function send(res, status, body, headers = {}) {
  res.writeHead(status, { ...cors, 'Content-Type': 'application/json', ...headers });
  res.end(JSON.stringify(body));
}
function fail(res, status, code, message, headers) {
  send(res, status, { error: { code, message, requestId: `req-${hex16()}` } }, headers);
}
async function readBody(req, limit = MAX_UPLOAD + 1024) {
  const chunks = []; let size = 0;
  for await (const c of req) { size += c.length; if (size > limit) throw new Error('too big'); chunks.push(c); }
  return Buffer.concat(chunks);
}
async function readJson(req) {
  const raw = await readBody(req, 64 * 1024);
  return raw.length ? JSON.parse(raw.toString('utf8')) : {};
}
const authed = (req) => db.tokens.has((req.headers.authorization ?? '').replace(/^Bearer /, ''));
const summary = (c) => ({ id: c.id, title: c.title, docIds: c.docIds, pinned: c.pinned, projectId: c.projectId, updatedAt: c.updatedAt });
function page(list, cursor, size) {
  const start = cursor ? Number(Buffer.from(cursor, 'base64url').toString()) : 0;
  const next = start + size < list.length ? Buffer.from(String(start + size)).toString('base64url') : null;
  return { items: list.slice(start, start + size), nextCursor: next };
}

/** Returns the error [status, code, message] for an attachment list, or null when it is valid. */
function checkDocIds(docIds) {
  if (!Array.isArray(docIds) || docIds.length > config.maxAttachedDocs || docIds.some((d) => !ATTACH_ID.test(d))) {
    return [400, 'VALIDATION_FAILED', 'docIds'];
  }
  for (const a of docIds) {
    const [id, part] = a.split(':p');
    const doc = db.docs.get(id);
    if (doc?.status !== 'ready') return [404, 'DOC_NOT_FOUND', `unknown or non-ready doc ${a}`];
    if (part === undefined && doc.parts.length) return [400, 'VALIDATION_FAILED', `${a} has parts; attach a part`];
    if (part !== undefined && !doc.parts.some((p) => p.n === Number(part))) return [404, 'DOC_NOT_FOUND', `no part ${a}`];
  }
  return null;
}

// ---------- auth ----------
function issueToken(res) {
  const token = randomBytes(24).toString('base64url');
  db.tokens.add(token);
  return send(res, 200, { token, expiresAt: new Date(Date.now() + 43200_000).toISOString() });
}
async function verify(req, res) {
  const { code } = await readJson(req);
  if (!/^\d{6}$/.test(code ?? '')) return fail(res, 400, 'VALIDATION_FAILED', 'code must be 6 digits');
  if (ANY_CODE) return issueToken(res);
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
  return issueToken(res);
}

// ---------- docs ----------
async function createDoc(req, res) {
  const { fileName, size } = await readJson(req);
  if (typeof fileName !== 'string' || !fileName || fileName.length > 255 || !Number.isInteger(size) || size < 1 || size > MAX_UPLOAD) {
    return fail(res, 400, 'VALIDATION_FAILED', 'fileName and size are required');
  }
  const doc = { id: hex16(), title: fileName.replace(/\.pdf$/i, '').slice(0, 200), size, status: 'pending', pageCount: null, parts: [], createdAt: now() };
  db.docs.set(doc.id, doc);
  return send(res, 200, { doc, uploadUrl: `${API}/_s3/${doc.id}?sig=fake`, uploadHeaders: { 'Content-Type': 'application/pdf' } });
}

function confirmDoc(res, doc) {
  const file = db.files.get(doc.id);
  if (!file || file.length !== doc.size || file.subarray(0, 5).toString() !== '%PDF-') {
    return fail(res, 422, 'UPLOAD_INVALID', 'Not a PDF or size mismatch');
  }
  const isLarge = doc.size > SPLIT_BYTES || /large/i.test(doc.title);
  const willFail = /fail/i.test(doc.title);
  if (!isLarge && !willFail) {
    Object.assign(doc, { status: 'ready', pageCount: 12, parts: [] });
    return send(res, 200, { doc });
  }
  doc.status = 'processing';
  setTimeout(() => {
    if (!db.docs.has(doc.id)) return;
    if (willFail) { doc.status = 'failed'; return; }
    const parts = Array.from({ length: 6 }, (_, i) => ({ n: i + 1, startPage: i * 100 + 1, endPage: i * 100 + 100, size: Math.ceil(doc.size / 6) }));
    Object.assign(doc, { status: 'ready', pageCount: 600, parts });
  }, 3000);
  return send(res, 200, { doc });
}

function deleteDoc(res, id) {
  for (const chat of db.chats.values()) chat.docIds = chat.docIds.filter((a) => a.split(':p')[0] !== id);
  db.files.delete(id); db.docs.delete(id);
  return send(res, 200, { deleted: true });
}

async function docRoute(req, res, url, id, sub) {
  const m = req.method;
  const doc = isId(id) ? db.docs.get(id) : null;
  if (!doc) return fail(res, 404, 'DOC_NOT_FOUND', 'No such doc');
  if (sub === '/confirm' && m === 'POST') return confirmDoc(res, doc);
  if (sub === '/link' && m === 'GET') {
    const part = url.searchParams.get('part');
    if (part !== null && !doc.parts.some((p) => p.n === Number(part))) return fail(res, 404, 'DOC_NOT_FOUND', 'No such part');
    const q = part === null ? '' : `&part=${part}`;
    return send(res, 200, { url: `${API}/_file/${id}?sig=fake${q}`, expiresAt: new Date(Date.now() + 300_000).toISOString() });
  }
  if (!sub && m === 'PATCH') {
    const { title } = await readJson(req);
    if (typeof title !== 'string' || !title || title.length > 200) return fail(res, 400, 'VALIDATION_FAILED', 'title');
    doc.title = title;
    return send(res, 200, { doc });
  }
  if (!sub && m === 'DELETE') return deleteDoc(res, id);
  return fail(res, 404, 'VALIDATION_FAILED', `No route ${m}`);
}

// ---------- projects ----------
const projectList = () => [...db.projects.values()].sort((a, b) => a.name.localeCompare(b.name));
const validName = (name) => typeof name === 'string' && name.trim().length >= 1 && name.length <= 80;

async function projectsRoute(req, res, id) {
  const m = req.method;
  if (!id && m === 'GET') return send(res, 200, { items: projectList() });
  if (!id && m === 'POST') {
    const { name } = await readJson(req);
    if (!validName(name)) return fail(res, 400, 'VALIDATION_FAILED', 'name 1..80');
    if (db.projects.size >= PROJECT_CAP) return fail(res, 409, 'PROJECT_LIMIT_REACHED', 'Project limit reached');
    const project = { id: hex16(), name: name.trim(), createdAt: now(), updatedAt: now() };
    db.projects.set(project.id, project);
    return send(res, 200, { project });
  }
  const project = db.projects.get(id);
  if (!project) return fail(res, 404, 'PROJECT_NOT_FOUND', 'No such project');
  if (m === 'PATCH') {
    const { name } = await readJson(req);
    if (!validName(name)) return fail(res, 400, 'VALIDATION_FAILED', 'name 1..80');
    Object.assign(project, { name: name.trim(), updatedAt: now() });
    return send(res, 200, { project });
  }
  if (m === 'DELETE') {
    for (const chat of db.chats.values()) if (chat.projectId === id) chat.projectId = null;
    db.projects.delete(id);
    return send(res, 200, { deleted: true });
  }
  return fail(res, 404, 'VALIDATION_FAILED', `No route ${m}`);
}

// ---------- chats ----------
async function chatPatch(req, res, chat) {
  const body = await readJson(req);
  const allowed = ['title', 'docIds', 'pinned', 'projectId', ...(config.features?.statutes ? ['statutes'] : [])];
  const fields = allowed.filter((k) => body[k] !== undefined);
  if (!fields.length) return fail(res, 400, 'VALIDATION_FAILED', 'one field required');
  if (body.docIds !== undefined) {
    const bad = checkDocIds(body.docIds);
    if (bad) return fail(res, ...bad);
  }
  if (body.pinned !== undefined && typeof body.pinned !== 'boolean') return fail(res, 400, 'VALIDATION_FAILED', 'pinned');
  if (fields.includes('statutes') && typeof body.statutes !== 'boolean') return fail(res, 400, 'VALIDATION_FAILED', 'statutes');
  if (body.projectId !== undefined && body.projectId !== null && !db.projects.has(body.projectId)) {
    return fail(res, 404, 'PROJECT_NOT_FOUND', 'No such project');
  }
  if (body.title !== undefined && (typeof body.title !== 'string' || !body.title.trim() || body.title.length > 120)) {
    return fail(res, 400, 'VALIDATION_FAILED', 'title 1..120');
  }
  for (const k of fields) chat[k] = k === 'title' ? body.title.trim() : body[k];
  chat.updatedAt = now();
  return send(res, 200, { chat });
}

/** Maps an attachment id to the doc block the model would see: parent doc + page offset. */
function attachedBlocks(chat) {
  return chat.docIds.map((a) => {
    const [id, part] = a.split(':p');
    const doc = db.docs.get(id);
    if (!doc) return null;
    const p = part === undefined ? null : doc.parts.find((x) => x.n === Number(part));
    return { doc, offset: p ? p.startPage - 1 : 0 };
  }).filter(Boolean);
}

async function askRoute(req, res, chat) {
  const started = Date.now();
  const { text, model = config.model } = await readJson(req);
  if (typeof text !== 'string' || !text.length || text.length > 8000) return fail(res, 400, 'VALIDATION_FAILED', 'text');
  if (typeof model !== 'string' || model.length > 64 || !models.some((m) => m.id === model)) {
    return fail(res, 400, 'VALIDATION_FAILED', 'model is not allowed');
  }
  db.lastAskBody = { text, model };
  const msgs = db.messages.get(chat.id);
  if (!msgs.length) chat.title = text.slice(0, 60);
  msgs.push({ id: ulid(), role: 'user', text, citations: [], usage: null, model: null, stopReason: null, durationMs: null, followUps: [], at: now() });
  res.writeHead(200, { ...cors, 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache' });
  const ev = (event, data) => res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
  const blocks = attachedBlocks(chat);
  const b0 = blocks[0] ?? null; const b1 = blocks[1] ?? b0;
  const cite = (n, b, p, quote) => ({ n, docId: b?.doc.id ?? null, docTitle: b?.doc.title ?? 'General knowledge', page: b ? b.offset + p : null, endPage: b ? b.offset + p : null, quote, kind: 'doc', sourceUrl: null });
  let answer = '';
  const citations = [];
  const say = async (t) => { await sleep(150); answer += t; ev('text', { text: t }); };
  const cited = (c) => { citations.push(c); answer += ` [${c.n}]`; ev('citation', c); };
  ev('search', { query: 'consent requisites contract void', count: 8 });
  await say('## Ruling\n\n'); await say('The Court held that the **contract is void** '); await say('for lack of consent.');
  cited(cite(1, b0, 1, 'There was no meeting of the minds between the parties.'));
  await say('\n\nThe ratio rests on Article 1318 of the Civil Code.');
  cited(cite(2, b1, 2, 'Article 1318. There is no contract unless the following requisites concur.'));
  cited(cite(3, b0, 12, 'Consent is manifested by the meeting of the offer and the acceptance.'));
  await say('\n\nProtection orders follow the statute.');
  cited({ n: 4, docId: null, docTitle: 'RA 9262 · Sec. 26', page: null, endPage: null, kind: 'law',
    quote: '[Amended] Battered woman syndrome as a defense. Victim-survivors who are found by the courts to be suffering from battered woman syndrome do not incur any criminal and civil liability.',
    sourceUrl: 'https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html' });
  const followUps = ['What are the requisites of a valid contract?', 'How does vitiated consent differ from no consent?', 'Which later cases applied this ruling?'];
  ev('followups', { followUps });
  const usage = { input: 1200, output: 64, cacheRead: 800, cacheWrite: 0 };
  addUsage(db.usage.today, usage); addUsage(db.usage.total, usage);
  const message = { id: ulid(), role: 'assistant', text: answer, citations, usage, model, stopReason: 'end_turn', durationMs: Date.now() - started, followUps, at: now() };
  msgs.push(message);
  chat.updatedAt = now();
  ev('done', { message, usage });
  res.end();
}

async function chatsRoute(req, res, url, id, sub) {
  const m = req.method;
  if (!id && m === 'GET') {
    const list = [...db.chats.values()].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).map(summary);
    return send(res, 200, page(list, url.searchParams.get('cursor'), 50));
  }
  if (!id && m === 'POST') {
    const { docIds = [] } = await readJson(req);
    const bad = checkDocIds(docIds);
    if (bad) return fail(res, ...bad);
    const chat = { id: hex16(), title: 'New chat', docIds, pinned: false, projectId: null, createdAt: now(), updatedAt: now(), ...(config.features?.statutes ? { statutes: true } : {}) };
    db.chats.set(chat.id, chat); db.messages.set(chat.id, []);
    return send(res, 200, { chat });
  }
  const chat = db.chats.get(id);
  if (!chat) return fail(res, 404, 'CHAT_NOT_FOUND', 'No such chat');
  if (sub === '/ask' && m === 'POST') return askRoute(req, res, chat);
  if (!sub && m === 'GET') return send(res, 200, { chat, messages: db.messages.get(id) });
  if (!sub && m === 'PATCH') return chatPatch(req, res, chat);
  if (!sub && m === 'DELETE') {
    db.messages.delete(id); db.chats.delete(id); // docs and usage stay, as in the contract
    return send(res, 200, { deleted: true });
  }
  return fail(res, 404, 'VALIDATION_FAILED', `No route ${m}`);
}

// ---------- router ----------
async function api(req, res) {
  const url = new URL(req.url, API);
  const p = url.pathname; const m = req.method;
  if (m === 'OPTIONS') { res.writeHead(204, cors); return res.end(); }
  if (p.startsWith('/_s3/') && m === 'PUT') {
    if (req.headers['content-type'] !== 'application/pdf') { res.writeHead(403, cors); return res.end('<Error>SignatureDoesNotMatch</Error>'); }
    db.files.set(p.slice(5), await readBody(req)); res.writeHead(200, cors); return res.end();
  }
  if (p.startsWith('/_file/') && m === 'GET') {
    const file = db.files.get(p.slice(7));
    if (!file) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'Content-Type': 'application/pdf' }); return res.end(file);
  }
  if (p === '/_last-ask' && m === 'GET') return send(res, 200, db.lastAskBody);
  if (p === '/_revoke' && m === 'POST') { db.tokens.clear(); res.writeHead(204, cors); return res.end(); }
  if (p === '/v1/auth/verify' && m === 'POST') return verify(req, res);
  if (!authed(req)) return fail(res, 401, 'UNAUTHENTICATED', 'Missing or invalid token');
  if (p === '/v1/config' && m === 'GET') return send(res, 200, config);
  if (p === '/v1/usage' && m === 'GET') return send(res, 200, db.usage);
  if (p === '/v1/docs' && m === 'GET') return send(res, 200, page([...db.docs.values()].reverse(), url.searchParams.get('cursor'), 100));
  if (p === '/v1/docs' && m === 'POST') return createDoc(req, res);
  let r = /^\/v1\/docs\/([^/]+)(\/confirm|\/link)?$/.exec(p);
  if (r) return docRoute(req, res, url, r[1], r[2]);
  r = /^\/v1\/projects(?:\/([^/]+))?$/.exec(p);
  if (r) return projectsRoute(req, res, r[1]);
  r = /^\/v1\/chats(?:\/([^/]+)(\/ask)?)?$/.exec(p);
  if (r) return chatsRoute(req, res, url, r[1], r[2]);
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
    res.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
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
createServer(wrap(web)).listen(WEB_PORT, () =>
  console.log(`mock web   ${WEB_ORIGIN}  (${ANY_CODE ? 'any 6-digit code unlocks' : 'TOTP code 123456'})  serving ${dist}`));
