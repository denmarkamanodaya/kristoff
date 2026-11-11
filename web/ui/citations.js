// Citations: grouping into sources (one per doc + page), inline chips in the rendered answer,
// source cards, and opening a cited PDF page (via the part that holds it for split docs).
// Contract: docs/contract.md (Citation; "v2" large PDFs, file delete).
import * as api from '../services/api.js';
import { icon } from '../services/icons.js';
import { esc, notify, pageLabel, shortTitle } from './dom.js';
import { state } from './state.js';
import { handleError } from './errors.js';
import { refreshTurns } from './thread.js';

/** True when the cited doc is known to be deleted (deleted here, link said so, or not in a full library). */
export function isDeleted(docId) {
  if (!docId) return false;
  if (state.deletedDocs.has(docId)) return true;
  return state.docsLoaded && !state.docsCursor && !state.docs.some((d) => d.id === docId);
}

const keyOf = (c) => `${c.docId || c.docTitle}|${c.page ?? ''}`;

/** Unique sources of an answer, in first-cited order: `[{ ...citation, num }]`. */
export function buildSources(citations = []) {
  const seen = new Map();
  for (const c of citations) if (!seen.has(keyOf(c))) seen.set(keyOf(c), { ...c, num: seen.size + 1 });
  return [...seen.values()];
}

const quoteOf = (c) => `${c.docTitle}${c.page ? `, ${pageLabel(c.page, c.endPage)}` : ''} — “${String(c.quote).trim()}”`;

function chipHtml(cs) {
  const first = cs[0];
  const more = cs.length > 1 ? `<span class="more">+${cs.length - 1}</span>` : '';
  const tip = esc(cs.map(quoteOf).join('\n\n'));
  if (first.docId && isDeleted(first.docId)) {
    return `<span class="cite-chip deleted" aria-disabled="true" title="${esc(first.docTitle)}">file deleted${more}</span>`;
  }
  const label = `${esc(shortTitle(first.docTitle))}${first.page ? ` · ${esc(pageLabel(first.page, first.endPage))}` : ''}`;
  if (!first.docId) return `<span class="cite-chip plain" title="${tip}">${label}${more}</span>`;
  return `<button type="button" class="cite-chip" data-doc="${esc(first.docId)}" data-page="${esc(first.page ?? '')}" title="${tip}">${label}${more}</button>`;
}

/**
 * Renders answer markdown (sanitised) and turns each run of ` [n]` markers into one chip.
 * @param text - Answer text with ` [n]` markers.
 * @param citations - The answer's citations (n ≥ 1).
 * @returns Safe HTML.
 */
export function renderAnswerHtml(text, citations = []) {
  const byN = new Map(citations.map((c) => [c.n, c]));
  const html = DOMPurify.sanitize(marked.parse(text || ''));
  return html.replace(/(?: ?\[\d+\])+/g, (run) => {
    const cs = [...run.matchAll(/\[(\d+)\]/g)].map((m) => byN.get(Number(m[1]))).filter(Boolean);
    const unique = [...new Map(cs.map((c) => [keyOf(c), c])).values()];
    return unique.length ? ` ${chipHtml(unique)}` : run;
  });
}

/** Markup of one source card (button that opens the page, or a disabled "file deleted" card). */
export function sourceCardHtml(s) {
  const where = pageLabel(s.page, s.endPage);
  const deleted = s.docId && isDeleted(s.docId);
  const head = `<span class="src-title">${icon('file-text')}<span>${esc(s.docTitle)}</span></span>${where ? `<span class="src-page">${esc(where)}</span>` : ''}`;
  const quote = `<span class="src-quote">“${esc(String(s.quote).trim())}”</span>`;
  if (!s.docId || deleted) {
    return `<li class="src-card${deleted ? ' deleted' : ''}"><div class="src-body">${head}${quote}${deleted ? '<span class="src-link">file deleted</span>' : ''}</div></li>`;
  }
  return `<li class="src-card"><button type="button" class="src-body" data-doc="${esc(s.docId)}" data-page="${esc(s.page ?? '')}">${head}${quote}<span class="src-link">Open ${where || 'file'} ${icon('external-link')}</span></button></li>`;
}

/** Plain-text source list for "Copy with sources". */
export const sourcesText = (citations) => buildSources(citations).map((s) => `${s.num}. ${s.docTitle}${s.page ? `, ${pageLabel(s.page, s.endPage)}` : ''} — “${String(s.quote).trim()}”`).join('\n');

/** Picks the link request and in-file page for an original page number (split docs use parts). */
function linkTarget(docId, page) {
  const doc = state.docs.find((d) => d.id === docId);
  const part = page && doc?.parts?.find((p) => page >= p.startPage && page <= p.endPage);
  return part ? { part: part.n, page: page - part.startPage + 1 } : { part: null, page };
}

/**
 * Opens a document in a new tab. The tab opens synchronously inside the click (popup
 * blockers allow that), then gets the short-lived presigned URL once the link call returns.
 * @param docId - Doc id.
 * @param page - Original page number, or null for the first page.
 */
export async function openDoc(docId, page) {
  const w = window.open('', '_blank');
  if (w) { w.opener = null; w.document.title = 'Opening…'; w.document.body.textContent = 'Opening document…'; }
  try {
    const t = linkTarget(docId, page);
    const { url } = await api.getDocLink(docId, t.part);
    const target = t.page ? `${url}#page=${t.page}` : url;
    if (w) w.location.href = target;
    else notify('Your browser blocked the new tab. Allow pop-ups for this site, then try again.');
  } catch (err) {
    w?.close();
    if (err?.code !== 'DOC_NOT_FOUND') { handleError(err); return; }
    state.deletedDocs.add(docId);
    notify('That file was deleted. Its citations now show "file deleted".');
    if (state.chat) refreshTurns();
  }
}

document.addEventListener('click', (e) => {
  const link = e.target.closest('[data-doc]');
  if (!link) return;
  e.preventDefault();
  openDoc(link.dataset.doc, Number(link.dataset.page) || null);
});
