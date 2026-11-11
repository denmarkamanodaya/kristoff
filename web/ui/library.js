// Library view, uploads (create → PUT → confirm) and the file ⋯ menu (Rename, Delete) shared by
// the Library and the composer's attach list. Large PDFs show "Processing…", then their parts.
// Contract: docs/contract.md "v2" (file delete, large PDFs).
import * as api from '../services/api.js';
import { icon } from '../services/icons.js';
import { $, $$, esc, kb, messageFor, pageLabel, upsert } from './dom.js';
import { MAX_FILE_NAME, MAX_UPLOAD_BYTES, NAME_MAX, state } from './state.js';
import { openMenu } from './menu.js';
import {
  askConfirm, confirmHtml, isConfirming, isRenaming, registerConfirm, registerRename, renameHtml, startRename,
} from './inline.js';
import { watchProcessing } from './data.js';
import { renderAll } from './views.js';
import { handleError } from './errors.js';
import { attachIds, setDocIds } from './composer.js';
import { refreshTurns } from './thread.js';

const DELETE_TEXT = "Delete this file? Answers that cite it will show 'file deleted'.";

/** Label for an attachment id: "Title" or "Title · pp. 101–200" for a part. */
export function attachLabel(id) {
  const [docId, part] = id.split(':p');
  const doc = state.docs.find((d) => d.id === docId);
  const title = doc?.title ?? 'File';
  const p = part === undefined ? null : doc?.parts?.find((x) => x.n === Number(part));
  return p ? `${title} · ${pageLabel(p.startPage, p.endPage)}` : title;
}

function statusHtml(d) {
  if (d.status === 'ready') return `<button type="button" class="open" data-doc="${esc(d.id)}">Open ${icon('external-link')}</button>`;
  if (d.status === 'processing') return `<span class="doc-state processing">${icon('loader-circle', 'spin')}Processing…</span>`;
  const why = d.status === 'failed' ? 'Processing failed' : 'Upload incomplete';
  return `<span class="doc-state failed">${why}</span><button type="button" class="link-btn" data-doc-delete="${esc(d.id)}" data-where="lib">Delete</button>`;
}

function metaHtml(d) {
  const bits = [kb(d.size), new Date(d.createdAt).toLocaleDateString()];
  if (d.pageCount) bits.push(`${d.pageCount} pages`);
  if (d.parts?.length) bits.push(`${d.parts.length} parts`);
  return bits.join(' · ');
}

/** Returns the ⋯ button for a doc row; `where` is 'lib' or 'pop'. */
export function docMenuButton(d, where) {
  return `<button type="button" class="row-more" data-doc-menu="${esc(d.id)}" data-where="${where}" aria-haspopup="menu" aria-expanded="false" aria-label="File options: ${esc(d.title)}">${icon('more-horizontal')}</button>`;
}

/** Renders the title cell of a doc row: the rename input while renaming, else the title. */
export function docTitleHtml(d, where, cls = 't') {
  return isRenaming('doc', d.id, where) ? renameHtml(d.title, 'File name', NAME_MAX.doc) : `<div class="${cls}">${esc(d.title)}</div>`;
}

function cardHtml(d) {
  if (isConfirming('doc', d.id, 'lib')) return `<div class="doc-card confirming">${confirmHtml(DELETE_TEXT)}</div>`;
  return `<div class="doc-card"><div class="doc-top"><div class="ico">PDF</div>${docMenuButton(d, 'lib')}</div>
    ${docTitleHtml(d, 'lib')}<div class="m">${metaHtml(d)}</div><div class="doc-foot">${statusHtml(d)}</div></div>`;
}

/** Renders the Library grid. */
export function renderLibrary() {
  $('#libGrid').innerHTML = state.docs.length ? state.docs.map(cardHtml).join('')
    : '<div class="empty-lib">No documents yet. Upload decisions, statutes or papers as PDFs.</div>';
  $('#moreDocs').hidden = !state.docsCursor;
}

/** Opens the file ⋯ menu. */
export function openDocMenu(btn, d, where) {
  const back = `[data-doc-menu="${CSS.escape(d.id)}"][data-where="${where}"]`;
  openMenu(btn, [
    { label: 'Rename', icon: 'pencil', action: () => startRename('doc', d.id, where, back) },
    { separator: true },
    { label: 'Delete', icon: 'trash-2', danger: true, action: () => askConfirm('doc', d.id, where, back) },
  ], `Options for ${d.title}`);
}

registerConfirm('doc', async (id) => {
  try {
    await api.deleteDoc(id);
  } catch (err) {
    if (err?.code !== 'DOC_NOT_FOUND') { handleError(err); return; } // already gone counts as deleted
  }
  state.docs = state.docs.filter((d) => d.id !== id);
  state.deletedDocs.add(id);
  const keep = (a) => a.split(':p')[0] !== id; // the server removed it from every chat too
  state.pendingDocIds = state.pendingDocIds.filter(keep);
  state.chats = state.chats.map((c) => ({ ...c, docIds: c.docIds.filter(keep) }));
  if (state.chat) { state.chat = { ...state.chat, docIds: state.chat.docIds.filter(keep) }; refreshTurns(); }
});
registerRename('doc', async (id, title) => {
  try {
    const { doc } = await api.renameDoc(id, title);
    upsert(state.docs, doc);
    if (state.chat) refreshTurns();
  } catch (err) {
    handleError(err);
  }
});

// ---------- uploads ----------
let uploadSeq = 0;
/** Renders the per-file upload rows (Library and attach list). */
export function renderUploads() {
  const html = state.uploads.map((u) => (u.error
    ? `<li class="upload failed"><span class="name" title="${esc(u.name)}">${esc(u.name)}</span><button type="button" data-dismiss="${u.key}" aria-label="Dismiss">${icon('x')}</button><span class="status">${esc(u.error)}</span></li>`
    : `<li class="upload"><span class="name" title="${esc(u.name)}">${esc(u.name)}</span><span class="status">${esc(u.stage)}</span><span class="bar"><i style="width:${Math.round(u.progress * 100)}%"></i></span></li>`)).join('');
  $$('.upload-list').forEach((l) => { l.innerHTML = html; });
}

/** Client check, create, PUT to S3, confirm. Returns the Doc (ready or processing), or null. */
async function uploadOne(file) {
  const u = { key: ++uploadSeq, name: file.name, stage: 'Preparing…', progress: 0, error: null };
  state.uploads.push(u); renderUploads();
  const fail = (msg) => { u.error = msg; renderUploads(); return null; };
  if (!/\.pdf$/i.test(file.name)) return fail('Only PDF files can be uploaded.');
  if (file.name.length > MAX_FILE_NAME) return fail('The file name is too long (255 characters at most).');
  if (file.size < 1) return fail('The file is empty.');
  if (file.size > MAX_UPLOAD_BYTES) return fail(`The file is ${kb(file.size)}. The limit is 200 MB.`);
  try {
    const { doc, uploadUrl, uploadHeaders } = await api.createDoc(file.name, file.size);
    await api.uploadFile(uploadUrl, uploadHeaders, file, (f) => {
      u.progress = f; u.stage = `Uploading… ${Math.round(f * 100)}%`; renderUploads();
    });
    u.progress = 1; u.stage = 'Checking PDF…'; renderUploads();
    const { doc: confirmed } = await api.confirmDoc(doc.id);
    upsert(state.docs, confirmed);
    state.uploads = state.uploads.filter((x) => x !== u); renderUploads();
    return confirmed;
  } catch (err) {
    if (err?.code === 'UNAUTHENTICATED') return fail('Locked before the upload finished. Unlock and upload again.');
    return fail(messageFor(err));
  }
}

/**
 * Uploads files; when `attach` is true, attaches the ready ones that have no parts.
 * @param files - File objects from an input.
 * @param attach - True when the upload started from the composer.
 */
export async function uploadFiles(files, attach) {
  if (!files.length) return;
  const added = (await Promise.all(files.map(uploadOne))).filter(Boolean);
  renderAll();
  watchProcessing();
  const whole = added.filter((d) => d.status === 'ready' && !d.parts?.length).map((d) => d.id);
  if (attach && whole.length) await setDocIds([...attachIds(), ...whole]);
}

document.addEventListener('click', (e) => {
  const dismiss = e.target.closest('[data-dismiss]');
  if (dismiss) {
    state.uploads = state.uploads.filter((u) => String(u.key) !== dismiss.dataset.dismiss);
    renderUploads();
    return;
  }
  const menuBtn = e.target.closest('[data-doc-menu]');
  if (menuBtn) {
    const d = state.docs.find((x) => x.id === menuBtn.dataset.docMenu);
    if (d) openDocMenu(menuBtn, d, menuBtn.dataset.where);
    return;
  }
  const del = e.target.closest('[data-doc-delete]');
  if (del) askConfirm('doc', del.dataset.docDelete, del.dataset.where, null);
});
document.addEventListener('change', (e) => {
  if (!e.target.matches('.fileInput')) return;
  uploadFiles([...e.target.files], e.target.dataset.attach === '1').catch(handleError);
  e.target.value = '';
});
$('#libUpload').addEventListener('click', () => $('#libFile').click());
