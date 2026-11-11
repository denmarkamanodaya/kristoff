const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const state = { model: '', chats: [], docs: [], chat: null, pendingDocIds: [], busy: false, view: 'home' };

// ---------- helpers ----------
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const fmt = (n) => (n || 0).toLocaleString();
const kb = (b) => (b > 1048576 ? `${(b / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);

async function api(url, opts = {}) {
  const headers = opts.body && !(opts.body instanceof FormData) ? { 'Content-Type': 'application/json' } : {};
  const r = await fetch(url, { ...opts, headers });
  const data = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(data.error || r.statusText);
  return data;
}
const fileUrl = (docId, page) => `/api/docs/${docId}/file${page ? `#page=${page}` : ''}`;

// Group citations into unique source cards (one per document page).
function buildSources(citations = []) {
  const sources = []; const index = {};
  citations.forEach((c) => {
    const key = `${c.docId || c.doc}|${c.page ?? ''}`;
    if (!(key in index)) { index[key] = sources.length; sources.push({ ...c, num: sources.length + 1 }); }
    c.src = index[key] + 1;
  });
  return sources;
}

function renderMarkdown(text, citations = []) {
  buildSources(citations);
  const html = DOMPurify.sanitize(marked.parse(text || ''));
  return html.replace(/ ?\[(\d+)\]/g, (m, n) => {
    const c = citations[Number(n) - 1];
    if (!c) return m;
    const tip = `${c.doc}${c.page ? `, p. ${c.page}` : ''} — “${c.quote.trim()}”`;
    return `<a class="cite" title="${esc(tip)}" href="${c.docId ? fileUrl(c.docId, c.page) : '#'}" target="_blank">${c.src}</a>`;
  });
}

function sourcesHtml(citations) {
  const sources = buildSources(citations);
  if (!sources.length) return '';
  return `<div class="label"><svg viewBox="0 0 24 24"><path d="M4 5h16M4 12h16M4 19h10"/></svg>Sources</div>
  <div class="sources-row">${sources.map((s) => `
    <a class="src" href="${s.docId ? fileUrl(s.docId, s.page) : '#'}" target="_blank" title="${esc(s.quote.trim())}">
      <div class="snip">${esc(s.quote.trim())}</div>
      <div class="meta"><b>${s.num}</b><span>${esc(s.doc)}${s.page ? ` · p. ${s.page}` : ''}</span></div>
    </a>`).join('')}</div>`;
}

// ---------- composer ----------
const composer = $('#composerTpl').content.firstElementChild.cloneNode(true);
const ta = $('textarea', composer);
const pop = $('.attach-pop', composer);
const currentDocIds = () => (state.chat ? state.chat.docIds : state.pendingDocIds);

function renderChips() {
  const ids = currentDocIds();
  $('.chips', composer).innerHTML = ids.map((id) => {
    const d = state.docs.find((x) => x.id === id);
    return d ? `<span class="chip" title="${esc(d.title)}"><span>📄 ${esc(d.title)}</span><button type="button" data-rm="${id}" title="Remove">×</button></span>` : '';
  }).join('');
  $('.attach-btn', composer).classList.toggle('on', ids.length > 0);
  $('.attach-btn', composer).lastChild.textContent = ids.length ? `Attach (${ids.length})` : 'Attach';
  const n = ids.length;
  $('#threadDocs').textContent = n ? `${n} document${n > 1 ? 's' : ''} attached` : 'No documents attached';
}
function renderPop() {
  const ids = new Set(currentDocIds());
  $('.pop-list', composer).innerHTML = state.docs.length
    ? state.docs.map((d) => `<li data-id="${d.id}"><input type="checkbox" ${ids.has(d.id) ? 'checked' : ''}><span>${esc(d.title)}</span></li>`).join('')
    : '<div class="pop-empty">No documents yet. Upload a PDF to get started.</div>';
}
async function setDocIds(ids) {
  ids = [...new Set(ids)];
  if (state.chat) state.chat = { ...state.chat, ...(await api(`/api/chats/${state.chat.id}`, { method: 'PATCH', body: JSON.stringify({ docIds: ids }) })) };
  else state.pendingDocIds = ids;
  renderChips(); renderPop();
}
$('.attach-btn', composer).onclick = () => { pop.hidden = !pop.hidden; if (!pop.hidden) renderPop(); };
document.addEventListener('click', (e) => { if (!pop.hidden && !composer.contains(e.target)) pop.hidden = true; });
$('.pop-list', composer).onclick = (e) => {
  const li = e.target.closest('li[data-id]'); if (!li) return;
  const ids = currentDocIds(); const id = li.dataset.id;
  setDocIds(ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]);
};
$('.chips', composer).onclick = (e) => { const b = e.target.closest('[data-rm]'); if (b) setDocIds(currentDocIds().filter((x) => x !== b.dataset.rm)); };

const autosize = () => { ta.style.height = 'auto'; ta.style.height = `${ta.scrollHeight}px`; $('.send', composer).disabled = state.busy || !ta.value.trim(); };
ta.addEventListener('input', autosize);
ta.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); composer.requestSubmit(); } });
composer.onsubmit = (e) => { e.preventDefault(); const t = ta.value.trim(); if (!t || state.busy) return; ta.value = ''; autosize(); pop.hidden = true; ask(t); };

// ---------- uploads ----------
async function uploadFiles(files, attach) {
  if (!files.length) return;
  const fd = new FormData();
  files.forEach((f) => fd.append('files', f));
  const labels = $$('.fileInput').map((i) => i.parentElement);
  labels.forEach((l) => (l.firstChild.textContent = 'Uploading…'));
  try {
    const added = await api('/api/docs', { method: 'POST', body: fd });
    state.docs = await api('/api/docs');
    if (attach) await setDocIds([...currentDocIds(), ...added.map((d) => d.id)]);
    renderPop(); renderLibrary();
  } catch (err) { alert(`Upload failed: ${err.message}`); }
  finally { labels.forEach((l) => (l.firstChild.textContent = 'Upload PDF')); }
}
document.addEventListener('change', (e) => {
  if (!e.target.matches('.fileInput')) return;
  uploadFiles([...e.target.files], composer.contains(e.target));
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
  renderChips(); autosize();
  if (view !== 'library') setTimeout(() => ta.focus(), 0);
}

function renderThreads() {
  $('#threadList').innerHTML = state.chats.map((c) =>
    `<li data-id="${c.id}" class="${c.id === state.chat?.id ? 'active' : ''}" title="${esc(c.title)}">${esc(c.title)}</li>`).join('')
    || '<li class="muted" style="cursor:default">No threads yet</li>';
}

function renderLibrary() {
  $('#libGrid').innerHTML = state.docs.length ? [...state.docs].reverse().map((d) => `
    <div class="doc-card">
      <div class="ico">PDF</div>
      <div class="t">${esc(d.title)}</div>
      <div class="m">${kb(d.size)} · ${new Date(d.createdAt).toLocaleDateString()}</div>
      <a href="${fileUrl(d.id)}" target="_blank">Open ↗</a>
    </div>`).join('') : '<div class="empty-lib">No documents yet. Upload decisions, statutes or papers as PDFs.</div>';
}

async function renderUsage(totals) {
  const u = totals || await api('/api/usage');
  $('#usage').innerHTML = `Tokens · in ${fmt(u.input)} · out ${fmt(u.output)}<br>cached ${fmt(u.cacheRead)} · cache write ${fmt(u.cacheWrite)}`;
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
    $('.actions', el).innerHTML = `<button data-copy><svg viewBox="0 0 24 24"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/></svg>Copy</button>
      ${u ? `<span class="tokens">${fmt((u.input_tokens || 0) + (u.cache_read_input_tokens || 0) + (u.cache_creation_input_tokens || 0))} in · ${fmt(u.output_tokens)} out</span>` : ''}`;
    $('[data-copy]', el).onclick = (e) => { navigator.clipboard.writeText(a.text.replace(/ \[\d+\]/g, '')); e.currentTarget.lastChild.textContent = 'Copied'; };
  }
}

function renderThread() {
  $('#threadTitle').textContent = state.chat.title;
  const box = $('#turns'); box.innerHTML = '';
  const m = state.chat.messages;
  for (let i = 0; i < m.length; i += 2) box.append(turnEl(m[i], m[i + 1]));
  requestAnimationFrame(() => { $('#thread').scrollTop = $('#thread').scrollHeight; });
}

async function refreshThreads() { state.chats = await api('/api/chats'); renderThreads(); }

async function openThread(id) {
  state.chat = await api(`/api/chats/${id}`);
  renderThreads(); show('thread'); renderThread();
}

function newThread() {
  state.chat = null; state.pendingDocIds = [];
  renderThreads(); show('home');
}

// ---------- asking ----------
async function ask(text) {
  if (!state.chat) {
    state.chat = await api('/api/chats', { method: 'POST' });
    if (state.pendingDocIds.length) await setDocIds(state.pendingDocIds);
    state.pendingDocIds = [];
    state.chat.title = text.slice(0, 60);
    show('thread'); renderThread();
  }
  state.busy = true; autosize();
  const q = { role: 'user', text };
  const a = { role: 'assistant', text: '', citations: [] };
  state.chat.messages.push(q);
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
    const res = await fetch(`/api/chats/${state.chat.id}/messages`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ text }),
    });
    if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || res.statusText);
    const reader = res.body.getReader(); const dec = new TextDecoder(); let buf = '';
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      let i;
      while ((i = buf.indexOf('\n\n')) >= 0) {
        const raw = buf.slice(0, i); buf = buf.slice(i + 2);
        const ev = /^event: (.+)$/m.exec(raw)?.[1];
        const data = JSON.parse(/^data: (.+)$/m.exec(raw)?.[1] || '{}');
        if (ev === 'text') { a.text += data.text; paint(); }
        else if (ev === 'citation') { a.citations.push(data); a.text += ` [${data.n}]`; paint(); }
        else if (ev === 'done') { a.usage = data.usage; renderUsage(data.totals); }
        else if (ev === 'error') { a.text += `\n\n<span class="error">Error: ${esc(data.error)}</span>`; paint(); }
      }
    }
  } catch (e) {
    a.text += `\n\n<span class="error">Error: ${esc(e.message)}</span>`;
  } finally {
    finished = true;
    if (!a.text) a.text = '_(no answer)_';
    fillAnswer(el, a, true);
    state.chat.messages.push(a);
    state.busy = false; autosize();
    refreshThreads();
  }
}

// ---------- wiring ----------
$('#newThread').onclick = newThread;
$$('.nav-item').forEach((a) => (a.onclick = () => (a.dataset.view === 'home' ? newThread() : (renderLibrary(), show('library')))));
$('#threadList').onclick = (e) => { const li = e.target.closest('li[data-id]'); if (li) openThread(li.dataset.id); };
$('.suggestions').onclick = (e) => { const b = e.target.closest('button'); if (b) { ta.value = b.dataset.q; autosize(); ta.focus(); } };
document.addEventListener('keydown', (e) => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k' ) { e.preventDefault(); newThread(); } });

async function boot() {
  const cfg = await api('/api/config');
  state.model = cfg.model; $('.model-tag', composer).textContent = cfg.model;
  state.docs = await api('/api/docs');
  await refreshThreads();
  renderUsage();
  newThread();
}

boot();
