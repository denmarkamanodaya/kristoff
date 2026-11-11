// Kristoff — local legal research assistant powered by the Claude API.
require('dotenv').config({ quiet: true });
const express = require('express');
const multer = require('multer');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const Anthropic = require('@anthropic-ai/sdk');

const PORT = Number(process.env.PORT || 3333);
const HOST = process.env.HOST || '127.0.0.1';
const MODEL = process.env.KRISTOFF_MODEL || 'claude-sonnet-5-5';
const MAX_TOKENS = Number(process.env.KRISTOFF_MAX_TOKENS || 4096);

const DATA = path.join(__dirname, 'data');
const DOCS = path.join(DATA, 'docs');
const CHATS = path.join(DATA, 'chats');
for (const d of [DATA, DOCS, CHATS]) fs.mkdirSync(d, { recursive: true });

if (!process.env.ANTHROPIC_API_KEY) {
  console.error('\n  Missing ANTHROPIC_API_KEY. Copy .env.example to .env and add your key.\n');
  process.exit(1);
}
const anthropic = new Anthropic();


const SYSTEM_PROMPT = `You are Kristoff, a legal research assistant for a Philippine law student.

How to work:
- When documents are attached, ground every legal claim in them and cite the specific passage. If the answer is not in the documents, say so plainly before offering general knowledge, and label that part as general knowledge.
- Never invent case names, G.R. numbers, dates, ponentes, statutes or quotations. If unsure, say you are unsure.
- For case analysis, default to: Facts, Issue(s), Ruling, Ratio decidendi, and Notable doctrines. Use IRAC when the student asks for problem-solving.
- Be clear about which jurisdiction a rule comes from. Assume Philippine law unless told otherwise.
- Remind the student to verify important citations against the official source (SC E-Library, Lawphil, Official Gazette) when the stakes are high.
- Write clearly and concisely. Use headings and bullet points for digests.`;

// ---------- tiny JSON store ----------
const DB_FILE = path.join(DATA, 'db.json');
function loadDb() {
  try { return JSON.parse(fs.readFileSync(DB_FILE, 'utf8')); }
  catch { return { docs: [], usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 } }; }
}
function saveDb(db) { fs.writeFileSync(DB_FILE, JSON.stringify(db, null, 2)); }
const id = () => crypto.randomBytes(8).toString('hex');
const chatFile = (cid) => path.join(CHATS, `${cid.replace(/[^a-f0-9]/g, '')}.json`);
function loadChat(cid) {
  try { return JSON.parse(fs.readFileSync(chatFile(cid), 'utf8')); } catch { return null; }
}
function saveChat(chat) { fs.writeFileSync(chatFile(chat.id), JSON.stringify(chat, null, 2)); }

// ---------- app ----------
const app = express();
app.use(express.json({ limit: '1mb' }));

app.use(express.static(path.join(__dirname, 'public')));
const NM = path.join(__dirname, 'node_modules');
const vendor = { 'marked.js': path.join(NM, 'marked/lib/marked.umd.js'), 'purify.js': path.join(NM, 'dompurify/dist/purify.min.js') };
app.get('/vendor/:file', (req, res) => (vendor[req.params.file] ? res.sendFile(vendor[req.params.file]) : res.status(404).end()));

app.get('/api/config', (req, res) => res.json({ model: MODEL }));

// Documents
const upload = multer({
  storage: multer.memoryStorage(),
  limits: { fileSize: 32 * 1024 * 1024 },
  fileFilter: (req, file, cb) => cb(null, file.mimetype === 'application/pdf' || /\.pdf$/i.test(file.originalname)),
});

app.get('/api/docs', (req, res) => res.json(loadDb().docs));

app.post('/api/docs', upload.array('files', 20), (req, res) => {
  if (!req.files || !req.files.length) return res.status(400).json({ error: 'Upload one or more PDF files.' });
  const db = loadDb();
  const added = [];
  for (const f of req.files) {
    if (f.buffer.subarray(0, 5).toString() !== '%PDF-') continue;
    const doc = { id: id(), title: f.originalname.replace(/\.pdf$/i, ''), size: f.size, createdAt: Date.now() };
    fs.writeFileSync(path.join(DOCS, `${doc.id}.pdf`), f.buffer);
    db.docs.push(doc);
    added.push(doc);
  }
  saveDb(db);
  if (!added.length) return res.status(400).json({ error: 'No valid PDF files found.' });
  res.json(added);
});

app.patch('/api/docs/:id', (req, res) => {
  const db = loadDb();
  const doc = db.docs.find((d) => d.id === req.params.id);
  if (!doc) return res.status(404).json({ error: 'Not found' });
  if (typeof req.body.title === 'string' && req.body.title.trim()) doc.title = req.body.title.trim().slice(0, 200);
  saveDb(db);
  res.json(doc);
});

app.get('/api/docs/:id/file', (req, res) => {
  const doc = loadDb().docs.find((d) => d.id === req.params.id);
  if (!doc) return res.status(404).end();
  res.type('application/pdf').sendFile(path.join(DOCS, `${doc.id}.pdf`));
});

// Chats
app.get('/api/chats', (req, res) => {
  const chats = fs.readdirSync(CHATS).filter((f) => f.endsWith('.json'))
    .map((f) => { try { return JSON.parse(fs.readFileSync(path.join(CHATS, f), 'utf8')); } catch { return null; } })
    .filter(Boolean)
    .map(({ id, title, updatedAt, docIds }) => ({ id, title, updatedAt, docIds }))
    .sort((a, b) => b.updatedAt - a.updatedAt);
  res.json(chats);
});

app.post('/api/chats', (req, res) => {
  const chat = { id: id(), title: 'New chat', docIds: [], messages: [], createdAt: Date.now(), updatedAt: Date.now() };
  saveChat(chat);
  res.json(chat);
});

app.get('/api/chats/:id', (req, res) => {
  const chat = loadChat(req.params.id);
  chat ? res.json(chat) : res.status(404).json({ error: 'Not found' });
});

app.patch('/api/chats/:id', (req, res) => {
  const chat = loadChat(req.params.id);
  if (!chat) return res.status(404).json({ error: 'Not found' });
  const db = loadDb();
  if (Array.isArray(req.body.docIds)) chat.docIds = req.body.docIds.filter((d) => db.docs.some((x) => x.id === d));
  if (typeof req.body.title === 'string' && req.body.title.trim()) chat.title = req.body.title.trim().slice(0, 120);
  chat.updatedAt = Date.now();
  saveChat(chat);
  res.json(chat);
});

app.get('/api/usage', (req, res) => res.json(loadDb().usage));

// Build the API request: attached PDFs go in the first user turn (cached), then the history.
function buildMessages(chat, docs) {
  const docBlocks = docs.map((d, i) => ({
    type: 'document',
    source: { type: 'base64', media_type: 'application/pdf', data: fs.readFileSync(path.join(DOCS, `${d.id}.pdf`)).toString('base64') },
    title: d.title,
    citations: { enabled: true },
    ...(i === docs.length - 1 ? { cache_control: { type: 'ephemeral' } } : {}),
  }));
  return chat.messages.map((m, i) => {
    if (i === 0 && m.role === 'user') return { role: 'user', content: [...docBlocks, { type: 'text', text: m.text }] };
    return { role: m.role, content: m.text || '(no answer)' };
  });
}

app.post('/api/chats/:id/messages', async (req, res) => {
  const chat = loadChat(req.params.id);
  if (!chat) return res.status(404).json({ error: 'Not found' });
  const text = String(req.body.text || '').trim();
  if (!text) return res.status(400).json({ error: 'Empty message' });

  const db = loadDb();
  const docs = chat.docIds.map((d) => db.docs.find((x) => x.id === d)).filter(Boolean);
  if (!chat.messages.length) chat.title = text.slice(0, 60);
  chat.messages.push({ role: 'user', text, at: Date.now() });

  res.set({ 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'keep-alive' });
  res.flushHeaders();
  const send = (event, data) => res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);

  const answer = { role: 'assistant', text: '', citations: [], at: Date.now() };
  let aborted = false;
  res.on('close', () => { aborted = true; });

  try {
    const stream = anthropic.messages.stream({
      model: MODEL,
      max_tokens: MAX_TOKENS,
      system: SYSTEM_PROMPT,
      messages: buildMessages(chat, docs),
    });
    res.on('close', () => { if (!res.writableFinished) stream.abort(); });

    for await (const ev of stream) {
      if (ev.type !== 'content_block_delta') continue;
      if (ev.delta.type === 'text_delta') {
        answer.text += ev.delta.text;
        send('text', { text: ev.delta.text });
      } else if (ev.delta.type === 'citations_delta') {
        const c = ev.delta.citation;
        const cite = {
          n: answer.citations.length + 1,
          doc: c.document_title || docs[c.document_index]?.title || 'Document',
          docId: docs[c.document_index]?.id,
          page: c.start_page_number ?? null,
          endPage: c.end_page_number != null ? c.end_page_number - 1 : null,
          quote: c.cited_text,
        };
        answer.citations.push(cite);
        answer.text += ` [${cite.n}]`;
        send('citation', cite);
      }
    }
    const final = await stream.finalMessage();
    const u = final.usage || {};
    db.usage.input += u.input_tokens || 0;
    db.usage.output += u.output_tokens || 0;
    db.usage.cacheRead += u.cache_read_input_tokens || 0;
    db.usage.cacheWrite += u.cache_creation_input_tokens || 0;
    saveDb(db);
    answer.usage = u;
    if (final.stop_reason === 'max_tokens') {
      answer.text += '\n\n_(Answer cut off at the length limit. Ask "continue" for the rest.)_';
    }
    send('done', { usage: u, totals: db.usage });
  } catch (err) {
    const msg = err?.error?.error?.message || err.message || 'Request failed';
    if (!aborted) send('error', { error: msg });
    if (!answer.text) answer.text = `_(Error: ${msg})_`;
  } finally {
    chat.messages.push(answer);
    chat.updatedAt = Date.now();
    saveChat(chat);
    res.end();
  }
});

app.listen(PORT, HOST, () => {
  console.log(`\n  Kristoff is running at http://${HOST === '0.0.0.0' ? 'localhost' : HOST}:${PORT}`);
  console.log(`  Model: ${MODEL}
`);
});
