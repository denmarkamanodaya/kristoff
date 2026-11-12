// The one composer, used on home and for follow-ups: a rounded box with the question on top and a
// bottom row with + (upload or pick from the library) on the left and Model ▾, mic and send on the
// right. The model choice lives in memory only (contract "Model choice"). On desktop
// (`config.features.statutes`) a Statutes pill sits next to + and is saved per chat (contract
// "Chat.statutes"); the web build never shows it.
import { hydrateIcons, icon } from '../services/icons.js';
import { $, esc, notify, pageLabel } from './dom.js';
import { state } from './state.js';
import { inMenu } from './menu.js';
import { confirmHtml, isConfirming } from './inline.js';
import { patchChat } from './data.js';
import { guard } from './errors.js';
import { attachLabel, docMenuButton, docTitleHtml, renderUploads } from './library.js';
import { ask } from './thread.js';
import { canDictate, startDictation } from './speech.js';

const composer = $('#composerTpl').content.firstElementChild.cloneNode(true);
hydrateIcons(composer);
const ta = $('textarea', composer);
const pop = $('.attach-pop', composer);
const plusBtn = $('.plus-btn', composer);
const micBtn = $('.mic-btn', composer);
const modelSelect = $('.model-select', composer);
const statPill = $('.stat-pill', composer);
const statNote = $('.stat-note', composer);
let dictation = null;

/** Whether statute search is on for the open thread (absent means true), or for the next new thread. */
export const statutesOn = () => (state.chat ? state.chat.statutes ?? true : state.pendingStatutes);

/** Flips Statutes: saved on the open chat, or kept for the next new thread (sent after createChat). */
async function toggleStatutes() {
  const next = !statutesOn();
  if (state.chat) await patchChat(state.chat.id, { statutes: next });
  else { state.pendingStatutes = next; renderComposer(); }
}

const lawCountFormat = new Intl.NumberFormat();

const isCount = (n) => Number.isInteger(n) && n >= 0;

/** The note beside the Statutes pill, from config counts (contract "Config: features"), or '' when unknown. */
function statutesNote(f = {}) {
  if (!isCount(f.lawCount)) return '';
  const laws = lawCountFormat.format(f.lawCount);
  if (f.cases === true && isCount(f.caseCount)) {
    return `Searches ${laws} laws and ${lawCountFormat.format(f.caseCount)} Supreme Court decisions on this Mac`;
  }
  return `Searches ${laws} Republic Acts and the 1987 Constitution on this Mac`;
}

function renderStatutes() {
  const shown = state.features?.statutes === true;
  const note = statutesNote(state.features);
  statPill.hidden = !shown;
  statNote.hidden = !shown || !note;
  statNote.textContent = note;
  statPill.setAttribute('aria-pressed', String(statutesOn()));
}

/** The attachment ids of the open thread, or of the next new thread. */
export const attachIds = () => (state.chat ? state.chat.docIds : state.pendingDocIds);

/**
 * Sets the attachments (deduplicated, capped at maxAttachedDocs) on the open chat or the draft.
 * @param ids - Attachment ids (`<docId>` or `<docId>:p<n>`).
 */
export async function setDocIds(ids) {
  let next = [...new Set(ids)];
  if (state.maxAttachedDocs && next.length > state.maxAttachedDocs) {
    notify(`A thread can have up to ${state.maxAttachedDocs} files or parts. Only the first ${state.maxAttachedDocs} were attached.`);
    next = next.slice(0, state.maxAttachedDocs);
  }
  if (state.chat) await patchChat(state.chat.id, { docIds: next });
  else { state.pendingDocIds = next; renderComposer(); }
}

const autosize = () => {
  ta.style.height = 'auto';
  ta.style.height = `${Math.min(ta.scrollHeight, 240)}px`;
  $('.send', composer).disabled = state.busy || !ta.value.trim();
};

/**
 * Moves the composer into a slot.
 * @param slot - The container element.
 * @param mode - 'home' or 'thread' (placeholder and popover direction).
 */
export function mountComposer(slot, mode) {
  slot.append(composer);
  composer.dataset.mode = mode;
  ta.placeholder = mode === 'thread' ? 'Ask a follow-up…' : 'Ask anything about your documents…';
  autosize();
  setTimeout(() => ta.focus(), 0);
}

/** Puts text in the box (suggestions) and focuses it. */
export function setDraft(text) { ta.value = text; autosize(); ta.focus(); }

function checkRow(id, label, on, off) {
  return `<label class="pick${off ? ' off' : ''}"><input type="checkbox" data-attach-id="${esc(id)}" ${on ? 'checked' : ''} ${off ? 'disabled' : ''}><span>${esc(label)}</span></label>`;
}

function pickHtml(d, ids, full) {
  if (isConfirming('doc', d.id, 'pop')) return `<li class="pick-doc confirming">${confirmHtml("Delete this file? Answers that cite it will show 'file deleted'.")}</li>`;
  const head = `<div class="pick-head">${icon('file-text')}${docTitleHtml(d, 'pop', 'pick-title')}${docMenuButton(d, 'pop')}</div>`;
  if (d.status === 'processing') return `<li class="pick-doc">${head}<div class="pick-note">${icon('loader-circle', 'spin')}Processing…</div></li>`;
  if (d.status !== 'ready') return `<li class="pick-doc">${head}<div class="pick-note failed">${d.status === 'failed' ? 'Processing failed' : 'Upload incomplete'} · <button type="button" class="link-btn" data-doc-delete="${esc(d.id)}" data-where="pop">Delete</button></div></li>`;
  if (!d.parts?.length) {
    const on = ids.has(d.id);
    return `<li class="pick-doc whole"><div class="pick-head">${checkRow(d.id, d.title, on, full && !on)}${docMenuButton(d, 'pop')}</div></li>`;
  }
  const parts = d.parts.map((p) => { const id = `${d.id}:p${p.n}`; const on = ids.has(id); return checkRow(id, pageLabel(p.startPage, p.endPage), on, full && !on); }).join('');
  return `<li class="pick-doc">${head}<div class="pick-parts" role="group" aria-label="Parts of ${esc(d.title)}">${parts}</div></li>`;
}

function renderPop() {
  const ids = new Set(attachIds());
  const full = state.maxAttachedDocs > 0 && ids.size >= state.maxAttachedDocs;
  $('.pop-hint', composer).textContent = state.docs.length && state.maxAttachedDocs ? `Up to ${state.maxAttachedDocs} files or parts per thread.` : '';
  $('.pop-list', composer).innerHTML = state.docs.length ? state.docs.map((d) => pickHtml(d, ids, full)).join('')
    : '<li class="pop-empty">No documents yet. Upload a PDF to get started.</li>';
}

/** Renders chips, the attach list, the model picker and the send state. */
export function renderComposer() {
  const ids = attachIds();
  $('.chips', composer).innerHTML = ids.map((id) => `<span class="chip" title="${esc(attachLabel(id))}">${icon('file-text')}<span>${esc(attachLabel(id))}</span><button type="button" data-rm="${esc(id)}" aria-label="Remove ${esc(attachLabel(id))}">${icon('x')}</button></span>`).join('');
  plusBtn.classList.toggle('on', ids.length > 0);
  renderStatutes();
  if (!pop.hidden) renderPop();
  renderUploads();
  autosize();
}

/** Fills the model picker from config and selects the current model. */
export function renderModelPicker() {
  const models = state.models.length ? state.models : [{ id: state.model, label: state.model }];
  modelSelect.innerHTML = models.map((m) => `<option value="${esc(m.id)}">${esc(m.label)}</option>`).join('');
  modelSelect.value = state.model;
  $('.model-pick', composer).hidden = !state.model;
}

function setPop(open) {
  pop.hidden = !open;
  plusBtn.setAttribute('aria-expanded', String(open));
  if (open) renderPop();
}

// ---------- dictation ----------
function stopDictation() { dictation?.stop(); }
function toggleDictation() {
  if (dictation) { stopDictation(); return; }
  const base = ta.value && !/\s$/.test(ta.value) ? `${ta.value} ` : ta.value;
  micBtn.classList.add('listening'); micBtn.setAttribute('aria-pressed', 'true'); micBtn.setAttribute('aria-label', 'Stop dictation');
  dictation = startDictation({
    onUpdate: (finalText, interim) => { ta.value = base + finalText + interim; autosize(); },
    onError: (code) => { if (code === 'not-allowed' || code === 'service-not-allowed') notify('Microphone access is blocked. Allow it in the browser to dictate.'); },
    onEnd: () => {
      dictation = null;
      micBtn.classList.remove('listening'); micBtn.setAttribute('aria-pressed', 'false'); micBtn.setAttribute('aria-label', 'Dictate');
      ta.value = ta.value.trimEnd(); autosize(); ta.focus();
    },
  });
}
micBtn.hidden = !canDictate;
micBtn.onclick = toggleDictation;

// ---------- wiring ----------
modelSelect.onchange = () => { state.model = modelSelect.value; };
plusBtn.onclick = () => setPop(pop.hidden);
statPill.onclick = guard(toggleStatutes);
$('.pop-upload', composer).onclick = () => $('.fileInput', composer).click();
pop.addEventListener('change', (e) => {
  const box = e.target.closest('[data-attach-id]');
  if (!box) return;
  const id = box.dataset.attachId;
  const ids = attachIds();
  guard(setDocIds)(box.checked ? [...ids, id] : ids.filter((x) => x !== id));
});
$('.chips', composer).onclick = (e) => {
  const b = e.target.closest('[data-rm]');
  if (b) guard(setDocIds)(attachIds().filter((x) => x !== b.dataset.rm));
};
document.addEventListener('mousedown', (e) => {
  if (!pop.hidden && !e.composedPath().includes(composer) && !inMenu(e.target)) setPop(false);
});
ta.addEventListener('input', autosize);
ta.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); composer.requestSubmit(); }
});
composer.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape' || state.confirm || state.renaming) return; // inline.js owns those
  if (dictation) { e.preventDefault(); stopDictation(); } else if (!pop.hidden) { e.preventDefault(); setPop(false); plusBtn.focus(); }
});
composer.onsubmit = (e) => {
  e.preventDefault();
  const text = ta.value.trim();
  if (!text || state.busy) return;
  stopDictation();
  ta.value = ''; autosize(); setPop(false);
  ask(text);
};
