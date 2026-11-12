// Thread view: question bubble, "Finished in Ns", answer with inline source chips, action row,
// a sticky collapsible Sources panel per answer, related follow-ups, and Answer | Sources tabs.
// Asking streams through services/api.js. Contract: docs/contract.md (ask stream; "v2" extras).
import * as api from '../services/api.js';
import { icon } from '../services/icons.js';
import { $, esc, fmt, messageFor } from './dom.js';
import { state } from './state.js';
import { openMenu } from './menu.js';
import { confirmHtml, isConfirming } from './inline.js';
import { refreshThreads, renderUsage } from './data.js';
import { show } from './views.js';
import { handleError } from './errors.js';
import { askDeleteChat } from './sidebar.js';
import { buildSources, renderAnswerHtml, sourceCardHtml, sourcesText } from './citations.js';
import { canSpeak, speak, stopSpeaking } from './speech.js';
import { attachIds } from './composer.js';

const modelLabel = (id) => state.models.find((m) => m.id === id)?.label ?? id;
let streamingEl = null; // the turn whose answer is still streaming
const plain = (text) => String(text).replace(/ \[\d+\]/g, '').replace(/<[^>]+>/g, '');

/** "Finished in 4.2s" / "Finished in 12s" / "Finished in 1m 05s". */
function finishedIn(ms) {
  const s = ms / 1000;
  if (s < 10) return `Finished in ${s.toFixed(1)}s`;
  if (s < 60) return `Finished in ${Math.round(s)}s`;
  return `Finished in ${Math.floor(s / 60)}m ${String(Math.round(s % 60)).padStart(2, '0')}s`;
}

/** Renders the thread header: title, tabs, attachment count and the Delete button / confirm. */
export function renderThreadHead() {
  $('#threadTitle').textContent = state.chat.title;
  const n = attachIds().length;
  $('#threadDocs').textContent = n ? `${n} file${n > 1 ? 's' : ''} attached` : 'No files attached';
  const confirming = isConfirming('chat', state.chat.id, 'head');
  $('#headConfirm').hidden = !confirming;
  $('#headConfirm').innerHTML = confirming ? confirmHtml("Delete this thread? This can't be undone.") : '';
  $('#deleteOpenThread').hidden = confirming;
  const total = state.chat.messages.reduce((k, m) => k + buildSources(m.citations).length, 0);
  $('#tabSources .count').textContent = total ? String(total) : '';
  for (const [tab, panel] of [['answer', '#turns'], ['sources', '#sourcesView']]) {
    const on = state.threadTab === tab;
    $(`#tab${tab === 'answer' ? 'Answer' : 'Sources'}`).setAttribute('aria-selected', String(on));
    $(panel).hidden = !on;
  }
  $('.follow-wrap').hidden = state.threadTab !== 'answer';
}

function sourcesPanelHtml(sources) {
  if (!sources.length) return '';
  const open = !state.sourcesCollapsed;
  return `<div class="srcs-head"><span>${icon('file-text')}Sources · ${sources.length}</span>
    <button type="button" class="icon-btn srcs-toggle" aria-expanded="${open}" aria-label="${open ? 'Collapse' : 'Expand'} sources">${icon(open ? 'chevron-down' : 'chevron-right')}</button></div>
    ${open ? `<ul class="src-list">${sources.map(sourceCardHtml).join('')}</ul>` : ''}`;
}

function actionsHtml(a, sources) {
  const stack = sources.slice(0, 3).map(() => `<span class="stack-ico">${icon('file-text')}</span>`).join('');
  return `<button type="button" class="act" data-act="copy">${icon('copy')}<span>Copy</span></button>
    ${canSpeak ? `<button type="button" class="act" data-act="read" aria-pressed="false">${icon('volume-2')}<span>Read aloud</span></button>` : ''}
    ${a.model ? `<span class="answer-model">${esc(modelLabel(a.model))}</span>` : ''}
    ${sources.length ? `<button type="button" class="act srcs-count" data-act="sources"><span class="stack">${stack}</span>${sources.length} source${sources.length > 1 ? 's' : ''}</button>` : ''}
    <button type="button" class="act icon-only" data-act="more" aria-haspopup="menu" aria-expanded="false" aria-label="More answer actions">${icon('more-horizontal')}</button>
    ${a.usage ? `<span class="tokens">${fmt(a.usage.input + a.usage.cacheRead + a.usage.cacheWrite)} in · ${fmt(a.usage.output)} out</span>` : ''}`;
}

function followUpsHtml(list = []) {
  if (!list.length) return '';
  return `<h3 class="fu-label">Related</h3><ul class="fu-list">${list.map((f) => `<li><button type="button" class="fu" data-followup="${esc(f)}">${icon('corner-down-right')}<span>${esc(f)}</span>${icon('plus', 'fu-plus')}</button></li>`).join('')}</ul>`;
}

/** "Searched PH laws: “query” → 8 articles" lines from the live `search` events (not persisted). */
function searchLineHtml(searches = []) {
  return searches.map((s) => {
    const n = Number(s?.count) || 0;
    return `<p>${icon('search')}<span>Searched PH laws: “${esc(String(s?.query ?? '').slice(0, 200))}” → ${n} article${n === 1 ? '' : 's'}</span></p>`;
  }).join('');
}

function turnEl(q) {
  const el = document.createElement('section');
  el.className = 'turn';
  el.innerHTML = `<div class="q-row"><div class="q-bubble">${esc(q.text)}</div></div>
    <div class="turn-grid"><div class="turn-main"><div class="meta-line"></div><div class="search-line"></div><div class="answer"></div><div class="actions"></div><div class="followups"></div></div>
    <aside class="turn-sources" aria-label="Sources for this answer"></aside></div>`;
  el.qText = q.text;
  return el;
}

function fillAnswer(el, a, final) {
  const sources = buildSources(a.citations);
  el.answer = a;
  $('.meta-line', el).innerHTML = final
    ? (a.durationMs != null ? esc(finishedIn(a.durationMs)) : '')
    : '<span class="dots"><i></i><i></i><i></i></span>Reading your documents…';
  $('.search-line', el).innerHTML = searchLineHtml(a.searches);
  $('.answer', el).innerHTML = a.text ? renderAnswerHtml(a.text, a.citations) : '';
  $('.answer', el).classList.toggle('cursor', !final);
  $('.turn-sources', el).innerHTML = sourcesPanelHtml(sources);
  $('.turn-grid', el).classList.toggle('no-sources', !sources.length);
  $('.actions', el).innerHTML = final ? actionsHtml(a, sources) : '';
  $('.followups', el).innerHTML = final ? followUpsHtml(a.followUps) : '';
}

function renderSourcesView() {
  const groups = [];
  let q = null;
  for (const m of state.chat.messages) {
    if (m.role === 'user') q = m.text;
    else if (buildSources(m.citations).length) groups.push({ q, sources: buildSources(m.citations) });
  }
  $('#sourcesView').innerHTML = groups.length
    ? groups.map((g) => `<section class="sv-group"><h3 class="sv-q">${esc(g.q)}</h3><ul class="src-list grid">${g.sources.map(sourceCardHtml).join('')}</ul></section>`).join('')
    : '<p class="muted sv-empty">No sources yet. Attach files and ask a question.</p>';
}

/** Re-fills the existing turns in place (sources panel toggle, a renamed or deleted file). */
export function refreshTurns() {
  document.querySelectorAll('#turns .turn').forEach((t) => { if (t.answer && t !== streamingEl) fillAnswer(t, t.answer, true); });
  renderSourcesView();
}

/** Renders every turn of the open chat from state. */
export function renderThread() {
  const box = $('#turns');
  box.innerHTML = '';
  let turn = null;
  for (const m of state.chat.messages) {
    if (m.role === 'user') { turn = turnEl(m); box.append(turn); } else if (turn) { fillAnswer(turn, m, true); turn = null; }
  }
  renderSourcesView();
  renderThreadHead();
  requestAnimationFrame(() => { $('#thread').scrollTop = $('#thread').scrollHeight; });
}

/**
 * Asks a question in the open chat (creating it first on home) and streams the answer.
 * @param text - The question, 1..8000 chars.
 */
export async function ask(text) {
  state.busy = true;
  if (!state.chat) {
    try {
      let { chat } = await api.createChat(state.pendingDocIds);
      // Chat.statutes defaults to true; send the draft's choice only when it differs (desktop only).
      if (state.features?.statutes && state.pendingStatutes !== (chat.statutes ?? true)) {
        ({ chat } = await api.updateChat(chat.id, { statutes: state.pendingStatutes }));
      }
      state.chat = { ...chat, messages: [], title: text.slice(0, 60) };
      state.pendingDocIds = []; state.threadTab = 'answer';
      show('thread');
    } catch (err) {
      state.busy = false;
      handleError(err);
      return;
    }
  }
  const chat = state.chat;
  const q = { role: 'user', text };
  let a = { role: 'assistant', text: '', citations: [], usage: null, followUps: [], durationMs: null, searches: [] };
  chat.messages.push(q);
  const el = turnEl(q);
  $('#turns').append(el);
  streamingEl = el;
  fillAnswer(el, a, false);
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
      onFollowUps: (list) => { a.followUps = list; },
      onSearch: (s) => { a.searches.push(s); paint(); },
    });
    a = { ...done.message, searches: a.searches }; // persisted message is the source of truth; searches are live-only
  } catch (err) {
    a.text += `\n\n<span class="error">Error: ${esc(messageFor(err))}</span>`;
    if (err?.code === 'CHAT_NOT_FOUND') refreshThreads().catch(handleError);
  } finally {
    finished = true;
    streamingEl = null;
    if (!a.text) a.text = '_(no answer)_';
    fillAnswer(el, a, true);
    chat.messages.push(a);
    state.busy = false;
    if (state.chat === chat) { renderSourcesView(); renderThreadHead(); }
    refreshThreads().catch(handleError);
    renderUsage().catch(handleError);
  }
}

// ---------- answer actions ----------
function readAloud(btn, el) {
  if (btn.getAttribute('aria-pressed') === 'true') { stopSpeaking(); return; }
  const copy = $('.answer', el).cloneNode(true);
  copy.querySelectorAll('.cite-chip').forEach((c) => c.remove());
  btn.setAttribute('aria-pressed', 'true'); btn.lastElementChild.textContent = 'Stop';
  speak(copy.textContent, () => { btn.setAttribute('aria-pressed', 'false'); btn.lastElementChild.textContent = 'Read aloud'; });
}

function copyText(btn, text) {
  const label = btn.querySelector('span');
  navigator.clipboard.writeText(text).then(() => { if (label) label.textContent = 'Copied'; }, () => { if (label) label.textContent = 'Copy failed'; });
}

function onAction(btn, el) {
  const a = el.answer;
  const act = btn.dataset.act;
  if (act === 'copy') copyText(btn, plain(a.text));
  else if (act === 'read') readAloud(btn, el);
  else if (act === 'sources') {
    state.sourcesCollapsed = false;
    fillAnswer(el, a, true);
    $('.turn-sources', el).scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    $('.turn-sources .src-body', el)?.focus();
  } else if (act === 'more') {
    openMenu(btn, [
      { label: 'Copy with sources', icon: 'copy', action: () => copyText(btn, `${plain(a.text)}\n\nSources:\n${sourcesText(a.citations)}`) },
      { label: 'Ask again', icon: 'rotate-ccw', disabled: state.busy, action: () => ask(el.qText) },
    ], 'Answer actions');
  }
}

$('#turns').addEventListener('click', (e) => {
  const el = e.target.closest('.turn');
  if (!el) return;
  const act = e.target.closest('[data-act]');
  if (act) { onAction(act, el); return; }
  if (e.target.closest('.srcs-toggle')) {
    state.sourcesCollapsed = !state.sourcesCollapsed;
    refreshTurns();
    $('.srcs-toggle', el)?.focus();
    return;
  }
  const fu = e.target.closest('[data-followup]');
  if (fu && !state.busy) ask(fu.dataset.followup);
});
$('.tabs').addEventListener('click', (e) => {
  const tab = e.target.closest('[role="tab"]');
  if (!tab) return;
  state.threadTab = tab.id === 'tabSources' ? 'sources' : 'answer';
  renderThreadHead();
});
$('.tabs').addEventListener('keydown', (e) => {
  if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
  state.threadTab = state.threadTab === 'answer' ? 'sources' : 'answer';
  renderThreadHead();
  $(state.threadTab === 'answer' ? '#tabAnswer' : '#tabSources').focus();
});
$('#deleteOpenThread').addEventListener('click', () => { if (state.chat) askDeleteChat(state.chat.id, 'head', '#deleteOpenThread'); });
