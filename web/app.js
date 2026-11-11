// Kristoff web UI. Rendering and event wiring only: every network call goes through
// services/api.js, the session lives in services/session.js (memory only), and the TOTP gate
// blocks the app until a code is verified. Contract: docs/contract.md.
import * as api from './services/api.js';
import * as auth from './services/auth.js';

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const state = {
  model: '', models: [], maxAttachedDocs: 0, chats: [], chatsCursor: null, docs: [], docsCursor: null,
  chat: null, pendingDocIds: [], busy: false, view: 'home', booted: false, uploads: [],
};
const MAX_UPLOAD_BYTES = 33554432; // contract: POST /v1/docs `size` 1..33554432
const MAX_FILE_NAME = 255; // contract: POST /v1/docs `fileName` ≤ 255

// ---------- helpers ----------
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const fmt = (n) => (n || 0).toLocaleString();
const kb = (b) => (b > 1048576 ? `${(b / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);
const upsert = (list, item) => { const i = list.findIndex((x) => x.id === item.id); if (i >= 0) list[i] = item; else list.unshift(item); };
const appendNew = (list, items) => { const seen = new Set(list.map((x) => x.id)); list.push(...items.filter((x) => !seen.has(x.id))); };

/** Turns an ApiError into the sentence the user sees, per the contract's client actions. */
function messageFor(err) {
  const ref = err?.requestId ? ` (ref ${err.requestId})` : '';
  switch (err?.code) {
    case 'OTP_INVALID': return 'That code is not right. Check your authenticator app and try again.';
    case 'OTP_REPLAYED': return 'That code was already used. Wait for the next code, then enter it.';
    case 'OTP_LOCKED': {
      const m = Math.max(1, Math.ceil((err.retryAfter ?? 900) / 60));
      return `Too many wrong codes. Try again in ${m} minute${m > 1 ? 's' : ''}.`;
    }
    case 'UNAUTHENTICATED': return 'Your session ended. Enter a new code.';
    case 'VALIDATION_FAILED': return `The request was not accepted: ${err.message}`;
    case 'DOC_NOT_FOUND': return 'A document could not be found. The library was refreshed.';
    case 'CHAT_NOT_FOUND': return 'That thread no longer exists. The thread list was refreshed.';
    case 'UPLOAD_INVALID': return 'The file was rejected: it is not a valid PDF, or its size did not match.';
    case 'DAILY_CAP_REACHED': return "You reached today's question limit. Try again tomorrow.";
    case 'AI_DISABLED': return 'Answers are switched off for now. Try again later.';
    case 'UPSTREAM_UNAVAILABLE':
    case 'INTERNAL': return `Something went wrong on the server. Try again in a moment.${ref}`;
    default: return (err?.message || 'Something went wrong.') + ref;
  }
}

/** Shows a dismissible inline notice above the current view. */
function notify(text) {
  const n = $('#notice');
  $('span', n).textContent = text;
  n.hidden = false;
}
$('#notice button').onclick = () => { $('#notice').hidden = true; };

/** Central error handler: inline notice plus the contract's resync action. */
function handleError(err) {
  if (err?.name === 'AbortError') return;
  if (err?.code === 'UNAUTHENTICATED') return; // the session expiry listener already shows the gate
  console.error(err);
  notify(messageFor(err));
  if (err?.code === 'DOC_NOT_FOUND') refreshDocs().catch(handleError);
  if (err?.code === 'CHAT_NOT_FOUND') { refreshThreads().catch(handleError); newThread(); }
}
const guard = (fn) => (...args) => Promise.resolve(fn(...args)).catch(handleError);

// ---------- auth gate ----------
const gate = $('#gate');
const otp = $('#otp');
const gateMsg = $('#gateMsg');
const gateBtn = $('.gate-btn');
let verifying = false;
let lockTimer = null;

function setGateEnabled(on) { otp.disabled = !on; gateBtn.disabled = !on; }

function showGate(msg = '') {
  gate.hidden = false;
  $('#app').hidden = true;
  otp.value = '';
  gateMsg.textContent = msg;
  if (!lockTimer) setGateEnabled(true);
  setTimeout(() => otp.focus(), 0);
}

function lockGate(seconds) {
  setGateEnabled(false);
  clearTimeout(lockTimer);
  lockTimer = setTimeout(() => { lockTimer = null; setGateEnabled(true); gateMsg.textContent = ''; otp.focus(); }, (seconds ?? 900) * 1000);
}

async function submitCode() {
  const code = otp.value;
  if (verifying || lockTimer) return;
  if (!auth.isCode(code)) { gateMsg.textContent = 'Enter all 6 digits.'; return; }
  verifying = true;
  setGateEnabled(false);
  gateMsg.textContent = '';
  try {
    await auth.verify(code);
    gate.hidden = true;
    $('#app').hidden = false;
    await (state.booted ? resync() : boot());
  } catch (err) {
    otp.value = '';
    gateMsg.textContent = messageFor(err);
    setGateEnabled(true);
    if (err?.code === 'OTP_LOCKED') lockGate(err.retryAfter);
    else otp.focus();
  } finally {
    verifying = false;
  }
}

otp.addEventListener('input', () => {
  const digits = otp.value.replace(/\D/g, '').slice(0, 6);
  if (digits !== otp.value) otp.value = digits;
  if (digits.length < 6) return;
  submitCode();
});
$('#gateForm').onsubmit = (e) => { e.preventDefault(); submitCode(); };
auth.onExpired(() => showGate('Your session ended. Enter a new code.'));

// ---------- citations and markdown ----------
// Group citations into unique source cards (one per document page).
function buildSources(citations = []) {
  const sources = []; const index = {};
  citations.forEach((c) => {
    const key = `${c.docId || c.docTitle}|${c.page ?? ''}`;
    if (!(key in index)) { index[key] = sources.length; sources.push({ ...c, num: sources.length + 1 }); }
    c.src = index[key] + 1;
  });
  return sources;
}
const docAttrs = (c) => (c.docId ? `data-doc="${esc(c.docId)}" data-page="${esc(c.page ?? '')}"` : '');
const pageLabel = (c) => (c.page ? (c.endPage && c.endPage !== c.page ? `pp. ${c.page}–${c.endPage}` : `p. ${c.page}`) : '');

function renderMarkdown(text, citations = []) {
  buildSources(citations);
  const byN = new Map(citations.map((c) => [c.n, c]));
  const html = DOMPurify.sanitize(marked.parse(text || ''));
  return html.replace(/ ?\[(\d+)\]/g, (m, n) => {
    const c = byN.get(Number(n));
    if (!c) return m;
    const where = pageLabel(c);
    const tip = `${c.docTitle}${where ? `, ${where}` : ''} — “${c.quote.trim()}”`;
    return `<a class="cite" href="#" title="${esc(tip)}" ${docAttrs(c)}>${c.src}</a>`;
  });
}

function sourcesHtml(citations) {
  const sources = buildSources(citations);
  if (!sources.length) return '';
  return `<div class="label"><svg viewBox="0 0 24 24"><path d="M4 5h16M4 12h16M4 19h10"/></svg>Sources</div>
  <div class="sources-row">${sources.map((s) => `
    <a class="src" href="#" ${docAttrs(s)} title="${esc(s.quote.trim())}">
      <div class="snip">${esc(s.quote.trim())}</div>
      <div class="meta"><b>${s.num}</b><span>${esc(s.docTitle)}${pageLabel(s) ? ` · ${pageLabel(s)}` : ''}</span></div>
    </a>`).join('')}</div>`;
}

/**
 * Opens a document in a new tab. The tab opens synchronously inside the click (popup
 * blockers allow that), then gets the short-lived presigned URL once the link call returns.
 */
async function openDoc(docId, page) {
  const w = window.open('', '_blank');
  if (w) { w.opener = null; w.document.title = 'Opening…'; w.document.body.textContent = 'Opening document…'; }
  try {
    const { url } = await api.getDocLink(docId);
    const target = page ? `${url}#page=${page}` : url;
    if (w) w.location.href = target;
    else notify('Your browser blocked the new tab. Allow pop-ups for this site, then try again.');
  } catch (err) {
    w?.close();
    handleError(err);
  }
}
document.addEventListener('click', (e) => {
  const link = e.target.closest('.cite, .src, [data-doc]');
  if (!link) return;
  e.preventDefault();
  if (link.dataset.doc) openDoc(link.dataset.doc, Number(link.dataset.page) || null);
});

// ---------- composer ----------
const composer = $('#composerTpl').content.firstElementChild.cloneNode(true);
const ta = $('textarea', composer);
const pop = $('.attach-pop', composer);
const currentDocIds = () => (state.chat ? state.chat.docIds : state.pendingDocIds);
const readyDocs = () => state.docs.filter((d) => d.status === 'ready');

function renderChips() {
  const ids = currentDocIds();
  $('.chips', composer).innerHTML = ids.map((id) => {
    const title = state.docs.find((x) => x.id === id)?.title ?? 'Document';
    return `<span class="chip" title="${esc(title)}"><span>📄 ${esc(title)}</span><button type="button" data-rm="${esc(id)}" title="Remove">×</button></span>`;
  }).join('');
  $('.attach-btn', composer).classList.toggle('on', ids.length > 0);
  $('.attach-btn', composer).lastChild.textContent = ids.length ? `Attach (${ids.length})` : 'Attach';
  const n = ids.length;
  $('#threadDocs').textContent = n ? `${n} document${n > 1 ? 's' : ''} attached` : 'No documents attached';
}
function renderPop() {
  const ids = new Set(currentDocIds());
  const full = state.maxAttachedDocs > 0 && ids.size >= state.maxAttachedDocs;
  const docs = readyDocs();
  $('.pop-hint', composer).textContent = docs.length && state.maxAttachedDocs ? `Up to ${state.maxAttachedDocs} documents per thread.` : '';
  $('.pop-list', composer).innerHTML = docs.length
    ? docs.map((d) => {
      const on = ids.has(d.id); const off = full && !on;
      return `<li data-id="${esc(d.id)}" class="${off ? 'off' : ''}"><input type="checkbox" ${on ? 'checked' : ''} ${off ? 'disabled' : ''}><span>${esc(d.title)}</span></li>`;
    }).join('')
    : '<div class="pop-empty">No documents yet. Upload a PDF to get started.</div>';
}
async function setDocIds(ids) {
  ids = [...new Set(ids)];
  if (state.maxAttachedDocs && ids.length > state.maxAttachedDocs) {
    notify(`A thread can have up to ${state.maxAttachedDocs} documents. Only the first ${state.maxAttachedDocs} were attached.`);
    ids = ids.slice(0, state.maxAttachedDocs);
  }
  if (state.chat) {
    const { chat } = await api.updateChat(state.chat.id, { docIds: ids });
    state.chat = { ...state.chat, ...chat };
  } else state.pendingDocIds = ids;
  renderChips(); renderPop();
}
$('.attach-btn', composer).onclick = () => { pop.hidden = !pop.hidden; if (!pop.hidden) renderPop(); };
document.addEventListener('click', (e) => { if (!pop.hidden && !e.composedPath().includes(composer)) pop.hidden = true; });
$('.pop-list', composer).onclick = guard((e) => {
  const li = e.target.closest('li[data-id]'); if (!li || li.classList.contains('off')) return undefined;
  e.preventDefault();
  const ids = currentDocIds(); const id = li.dataset.id;
  return setDocIds(ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]);
});
$('.chips', composer).onclick = guard((e) => {
  const b = e.target.closest('[data-rm]');
  return b ? setDocIds(currentDocIds().filter((x) => x !== b.dataset.rm)) : undefined;
});

// ---------- model picker (choice kept in memory only, docs/contract.md "Model choice") ----------
const modelSelect = $('.model-select', composer);
const modelLabel = (id) => state.models.find((m) => m.id === id)?.label ?? id;
function renderModelPicker() {
  const models = state.models.length ? state.models : [{ id: state.model, label: state.model }];
  modelSelect.innerHTML = models.map((m) => `<option value="${esc(m.id)}">${esc(m.label)}</option>`).join('');
  modelSelect.value = state.model;
  $('.model-pick', composer).hidden = !state.model;
}
modelSelect.onchange = () => { state.model = modelSelect.value; };

const autosize = () => { ta.style.height = 'auto'; ta.style.height = `${ta.scrollHeight}px`; $('.send', composer).disabled = state.busy || !ta.value.trim(); };
ta.addEventListener('input', autosize);
ta.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); composer.requestSubmit(); } });
composer.onsubmit = (e) => { e.preventDefault(); const t = ta.value.trim(); if (!t || state.busy) return; ta.value = ''; autosize(); pop.hidden = true; ask(t); };

// ---------- uploads ----------
let uploadSeq = 0;
function renderUploads() {
  const html = state.uploads.map((u) => (u.error
    ? `<li class="upload failed"><span class="name" title="${esc(u.name)}">${esc(u.name)}</span><button type="button" data-dismiss="${u.key}" title="Dismiss" aria-label="Dismiss">×</button><span class="status">${esc(u.error)}</span></li>`
    : `<li class="upload"><span class="name" title="${esc(u.name)}">${esc(u.name)}</span><span class="status">${esc(u.stage)}</span><span class="bar"><i style="width:${Math.round(u.progress * 100)}%"></i></span></li>`)).join('');
  $$('.upload-list').forEach((l) => { l.innerHTML = html; });
}
document.addEventListener('click', (e) => {
  const b = e.target.closest('[data-dismiss]'); if (!b) return;
  state.uploads = state.uploads.filter((u) => String(u.key) !== b.dataset.dismiss);
  renderUploads();
});

/** Client check, create, PUT to S3, confirm. Returns the ready Doc, or null when it failed. */
async function uploadOne(file) {
  const u = { key: ++uploadSeq, name: file.name, stage: 'Preparing…', progress: 0, error: null };
  state.uploads.push(u); renderUploads();
  const fail = (msg) => { u.error = msg; renderUploads(); return null; };
  if (!/\.pdf$/i.test(file.name)) return fail('Only PDF files can be uploaded.');
  if (file.name.length > MAX_FILE_NAME) return fail('The file name is too long (255 characters at most).');
  if (file.size < 1) return fail('The file is empty.');
  if (file.size > MAX_UPLOAD_BYTES) return fail(`The file is ${kb(file.size)}. The limit is 32 MB.`);
  try {
    const { doc, uploadUrl, uploadHeaders } = await api.createDoc(file.name, file.size);
    await api.uploadFile(uploadUrl, uploadHeaders, file, (f) => {
      u.progress = f; u.stage = `Uploading… ${Math.round(f * 100)}%`; renderUploads();
    });
    u.progress = 1; u.stage = 'Checking PDF…'; renderUploads();
    const { doc: ready } = await api.confirmDoc(doc.id);
    upsert(state.docs, ready);
    state.uploads = state.uploads.filter((x) => x !== u); renderUploads();
    return ready;
  } catch (err) {
    if (err?.code === 'UNAUTHENTICATED') return fail('Locked before the upload finished. Unlock and upload again.');
    return fail(messageFor(err));
  }
}
async function uploadFiles(files, attach) {
  if (!files.length) return;
  const added = (await Promise.all(files.map(uploadOne))).filter(Boolean);
  renderLibrary(); renderPop();
  if (attach && added.length) await setDocIds([...currentDocIds(), ...added.map((d) => d.id)]);
}
document.addEventListener('change', (e) => {
  if (!e.target.matches('.fileInput')) return;
  guard(uploadFiles)([...e.target.files], composer.contains(e.target));
  e.target.value = '';
});

// ---------- views ----------
function show(view) {
  state.view = view;
  ['home', 'thread', 'library'].forEach((v) => ($(`#${v}`).hidden = v !== view));
  $$('.nav-item').forEach((a) => a.classList.toggle('active', a.dataset.view === view));
  if (view === 'home') $('#homeSlot').append(composer);
  if (view === 'thread') $('#threadSlot').append(composer);
  ta.placeholder = view === 'thread' ? 'Ask a follow-up…' : 'Ask anything about your documents…';
  renderChips(); renderUploads(); autosize();
  if (view !== 'library') setTimeout(() => ta.focus(), 0);
}

function renderThreads() {
  $('#threadList').innerHTML = state.chats.map((c) =>
    `<li data-id="${esc(c.id)}" class="${c.id === state.chat?.id ? 'active' : ''}" title="${esc(c.title)}">${esc(c.title)}</li>`).join('')
    || '<li class="muted" style="cursor:default">No threads yet</li>';
  $('#moreThreads').hidden = !state.chatsCursor;
}

function renderLibrary() {
  $('#libGrid').innerHTML = state.docs.length ? state.docs.map((d) => `
    <div class="doc-card">
      <div class="ico">PDF</div>
      <div class="t">${esc(d.title)}</div>
      <div class="m">${kb(d.size)} · ${new Date(d.createdAt).toLocaleDateString()}</div>
      ${d.status === 'ready' ? `<button type="button" class="open" data-doc="${esc(d.id)}">Open ↗</button>` : '<span class="pending">Upload incomplete</span>'}
    </div>`).join('') : '<div class="empty-lib">No documents yet. Upload decisions, statutes or papers as PDFs.</div>';
  $('#moreDocs').hidden = !state.docsCursor;
}

async function renderUsage() {
  const { today: t, total: u } = await api.getUsage();
  $('#usage').innerHTML = `Today · in ${fmt(t.input)} · out ${fmt(t.output)}<br>Total · in ${fmt(u.input)} · out ${fmt(u.output)}<br>cached ${fmt(u.cacheRead)} · cache write ${fmt(u.cacheWrite)}`;
}

function turnEl(q, a) {
  const el = document.createElement('section');
  el.className = 'turn';
  el.innerHTML = `<h1 class="q">${esc(q.text)}</h1><div class="srcs"></div>
    <div class="label"><svg viewBox="0 0 24 24"><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"/></svg>Answer</div>
    <div class="answer"></div><div class="actions"></div>`;
  if (a) fillAnswer(el, a, true);
  return el;
}
function fillAnswer(el, a, final) {
  $('.srcs', el).innerHTML = sourcesHtml(a.citations);
  $('.answer', el).innerHTML = renderMarkdown(a.text, a.citations);
  $('.answer', el).classList.toggle('cursor', !final);
  if (final) {
    const u = a.usage;
    const modelTag = a.model ? `<span class="answer-model">${esc(modelLabel(a.model))}</span>` : '';
    $('.actions', el).innerHTML = `<button data-copy><svg viewBox="0 0 24 24"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/></svg>Copy</button>
      ${modelTag}${u ? `<span class="tokens">${fmt(u.input + u.cacheRead + u.cacheWrite)} in · ${fmt(u.output)} out</span>` : ''}`;
    $('[data-copy]', el).onclick = (e) => {
      const label = e.currentTarget.lastChild;
      navigator.clipboard.writeText(a.text.replace(/ \[\d+\]/g, ''))
        .then(() => { label.textContent = 'Copied'; }, () => { label.textContent = 'Copy failed'; });
    };
  }
}

function renderThread() {
  $('#threadTitle').textContent = state.chat.title;
  const box = $('#turns'); box.innerHTML = '';
  let turn = null;
  for (const m of state.chat.messages) {
    if (m.role === 'user') { turn = turnEl(m); box.append(turn); } else if (turn) { fillAnswer(turn, m, true); turn = null; }
  }
  requestAnimationFrame(() => { $('#thread').scrollTop = $('#thread').scrollHeight; });
}

async function refreshThreads() {
  const r = await api.listChats();
  state.chats = r.items; state.chatsCursor = r.nextCursor;
  renderThreads();
}
async function moreThreads() {
  const r = await api.listChats(state.chatsCursor);
  appendNew(state.chats, r.items); state.chatsCursor = r.nextCursor;
  renderThreads();
}
async function refreshDocs() {
  const r = await api.listDocs();
  state.docs = r.items; state.docsCursor = r.nextCursor;
  renderLibrary(); renderPop(); renderChips();
}
async function moreDocs() {
  const r = await api.listDocs(state.docsCursor);
  appendNew(state.docs, r.items); state.docsCursor = r.nextCursor;
  renderLibrary(); renderPop();
}

async function openThread(id) {
  const { chat, messages } = await api.getChat(id);
  state.chat = { ...chat, messages };
  renderThreads(); show('thread'); renderThread();
}

function newThread() {
  state.chat = null; state.pendingDocIds = [];
  renderThreads(); show('home');
}

// ---------- asking ----------
async function ask(text) {
  state.busy = true; autosize();
  if (!state.chat) {
    try {
      const { chat } = await api.createChat(state.pendingDocIds);
      state.chat = { ...chat, messages: [], title: text.slice(0, 60) };
      state.pendingDocIds = [];
      show('thread'); renderThread();
    } catch (err) {
      state.busy = false; ta.value = text; autosize();
      handleError(err);
      return;
    }
  }
  const chat = state.chat;
  const q = { role: 'user', text };
  let a = { role: 'assistant', text: '', citations: [], usage: null };
  chat.messages.push(q);
  const el = turnEl(q);
  $('.answer', el).innerHTML = '<div class="searching"><span class="dots"><i></i><i></i><i></i></span>Reading your documents…</div>';
  $('#turns').append(el);
  $('#thread').scrollTop = $('#thread').scrollHeight;

  let pending = false; let finished = false;
  const paint = () => {
    if (pending) return; pending = true;
    requestAnimationFrame(() => { pending = false; if (!finished) fillAnswer(el, a, false); });
  };

  try {
    const done = await api.askStream(chat.id, text, {
      model: state.model,
      onText: (t) => { a.text += t; paint(); },
      onCitation: (c) => { a.citations.push(c); a.text += ` [${c.n}]`; paint(); },
    });
    a = done.message; // the persisted message is the source of truth (same as a reload shows)
  } catch (err) {
    a.text += `\n\n<span class="error">Error: ${esc(messageFor(err))}</span>`;
    if (err?.code === 'CHAT_NOT_FOUND') refreshThreads().catch(handleError);
  } finally {
    finished = true;
    if (!a.text) a.text = '_(no answer)_';
    fillAnswer(el, a, true);
    chat.messages.push(a);
    state.busy = false; autosize();
    refreshThreads().catch(handleError);
    renderUsage().catch(handleError);
  }
}

// ---------- wiring ----------
$('#newThread').onclick = newThread;
$$('.nav-item').forEach((a) => (a.onclick = () => (a.dataset.view === 'home' ? newThread() : (renderLibrary(), show('library')))));
$('#threadList').onclick = guard((e) => { const li = e.target.closest('li[data-id]'); return li ? openThread(li.dataset.id) : undefined; });
$('#moreThreads').onclick = guard(moreThreads);
$('#moreDocs').onclick = guard(moreDocs);
$('.suggestions').onclick = (e) => { const b = e.target.closest('button'); if (b) { ta.value = b.dataset.q; autosize(); ta.focus(); } };
document.addEventListener('keydown', (e) => {
  if (!auth.isUnlocked()) return;
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); newThread(); }
});

/** First load after the first unlock: config, library, threads, usage. */
async function boot() {
  newThread();
  try {
    const cfg = await api.getConfig();
    state.model = cfg.model; state.models = cfg.models ?? []; state.maxAttachedDocs = cfg.maxAttachedDocs;
    renderModelPicker();
    await Promise.all([refreshDocs(), refreshThreads(), renderUsage()]);
    state.booted = true;
  } catch (err) {
    handleError(err);
  }
}

/** Reload after a re-unlock (session expiry): lists, usage and the open thread. */
async function resync() {
  try {
    await Promise.all([refreshDocs(), refreshThreads(), renderUsage()]);
    if (state.chat) await openThread(state.chat.id);
  } catch (err) {
    handleError(err);
  }
}

showGate();
