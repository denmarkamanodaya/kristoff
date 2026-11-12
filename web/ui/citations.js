// Citations: grouping into sources (one per doc + page), inline chips in the rendered answer,
// source cards (doc and law), and opening a cited PDF page (via the part that holds it for split docs) or a law source.
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

/** True for a statute citation (`kind:"law"`); a citation without `kind` is a doc citation. */
const isLaw = (c) => c.kind === 'law';
/** The law source link, only when it is an `https:` URL (anything else renders as plain text). */
const lawUrl = (c) => (isLaw(c) && /^https:\/\//i.test(c.sourceUrl ?? '') ? c.sourceUrl : null);
// Contract "Law status line": a law quote may start with one `[Status]` label (letters, spaces,
// hyphens, at most 40 chars) when the provision is not in force as written, e.g. `[Repealed] …`.
const STATUS_LINE = /^\s*\[([A-Za-z][A-Za-z -]{0,39})\]\s*/;

/** Splits a law quote into its status label (or null) and the quote text without it. */
export function lawStatus(c) {
  const quote = String(c.quote ?? '');
  const m = isLaw(c) ? STATUS_LINE.exec(quote) : null;
  return m ? { status: m[1].trim(), quote: quote.slice(m[0].length) } : { status: null, quote };
}
/** The quote as shown: law quotes lose their status line, doc quotes are unchanged. */
const shownQuote = (c) => lawStatus(c).quote.trim();

const keyOf = (c) => `${c.docId || c.docTitle}|${c.page ?? ''}`;

/** Unique sources of an answer, in first-cited order: `[{ ...citation, num }]`. */
export function buildSources(citations = []) {
  const seen = new Map();
  for (const c of citations) if (!seen.has(keyOf(c))) seen.set(keyOf(c), { ...c, num: seen.size + 1 });
  return [...seen.values()];
}

const statusSuffix = (c) => (lawStatus(c).status ? ` [${lawStatus(c).status}]` : '');
const quoteOf = (c) => `${c.docTitle}${statusSuffix(c)}${c.page ? `, ${pageLabel(c.page, c.endPage)}` : ''} — “${shownQuote(c)}”`;

function chipHtml(cs) {
  const first = cs[0];
  const more = cs.length > 1 ? `<span class="more">+${cs.length - 1}</span>` : '';
  const tip = esc(cs.map(quoteOf).join('\n\n'));
  if (first.docId && isDeleted(first.docId)) {
    return `<span class="cite-chip deleted" aria-disabled="true" title="${esc(first.docTitle)}">file deleted${more}</span>`;
  }
  const label = `${esc(shortTitle(first.docTitle))}${first.page ? ` · ${esc(pageLabel(first.page, first.endPage))}` : ''}`;
  if (lawUrl(first)) return `<button type="button" class="cite-chip law" data-src-url="${esc(lawUrl(first))}" title="${tip}">${label}${more}</button>`;
  if (isLaw(first)) return `<span class="cite-chip law plain" title="${tip}">${label}${more}</span>`;
  if (!first.docId) return `<span class="cite-chip plain" title="${tip}">${label}${more}</span>`;
  return `<button type="button" class="cite-chip doc" data-doc="${esc(first.docId)}" data-page="${esc(first.page ?? '')}" title="${tip}">${label}${more}</button>`;
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

/** Markup of a law source card (sand): title, status badge from the quote's status line, quote, official link. */
function lawCardHtml(s) {
  const { status } = lawStatus(s);
  const head = `<span class="src-title">${icon('library')}<span>${esc(s.docTitle)}</span></span><span class="src-badge">${status ? esc(status) : ''}</span>`;
  const quote = `<span class="src-quote">“${esc(shownQuote(s))}”</span>`;
  const url = lawUrl(s);
  if (!url) return `<li class="src-card law"><div class="src-body">${head}${quote}</div></li>`;
  return `<li class="src-card law"><button type="button" class="src-body" data-src-url="${esc(url)}">${head}${quote}<span class="src-link">Official text ${icon('external-link')}</span></button></li>`;
}

/** Markup of one source card (button that opens the page, or a disabled "file deleted" card). */
export function sourceCardHtml(s) {
  if (isLaw(s)) return lawCardHtml(s);
  const where = pageLabel(s.page, s.endPage);
  const deleted = s.docId && isDeleted(s.docId);
  const head = `<span class="src-title">${icon('file-text')}<span>${esc(s.docTitle)}</span></span>${where ? `<span class="src-page">${esc(where)}</span>` : ''}`;
  const quote = `<span class="src-quote">“${esc(shownQuote(s))}”</span>`;
  if (!s.docId || deleted) {
    return `<li class="src-card doc${deleted ? ' deleted' : ''}"><div class="src-body">${head}${quote}${deleted ? '<span class="src-link">file deleted</span>' : ''}</div></li>`;
  }
  return `<li class="src-card doc"><button type="button" class="src-body" data-doc="${esc(s.docId)}" data-page="${esc(s.page ?? '')}">${head}${quote}<span class="src-link">Open ${where || 'file'} ${icon('external-link')}</span></button></li>`;
}

/** Plain-text source list for "Copy with sources". */
export const sourcesText = (citations) => buildSources(citations).map((s) => `${s.num}. ${quoteOf(s)}`).join('\n');

/** Picks the link request and in-file page for an original page number (split docs use parts). */
function linkTarget(docId, page) {
  const doc = state.docs.find((d) => d.id === docId);
  const part = page && doc?.parts?.find((p) => page >= p.startPage && page <= p.endPage);
  return part ? { part: part.n, page: page - part.startPage + 1 } : { part: null, page };
}

/**
 * Opens a cited page through `api.openDoc` (D-12): a new tab on the web, the shell's viewer on
 * desktop. Must run synchronously inside the click so the web tab opens before any await.
 * @param docId - Doc id.
 * @param page - Original page number, or null for the first page.
 */
export async function openDoc(docId, page) {
  const t = linkTarget(docId, page);
  try {
    await api.openDoc(docId, t.part, t.page); // called before any await, so the web popup is allowed
  } catch (err) {
    if (err?.code === 'POPUP_BLOCKED') { notify('Your browser blocked the new tab. Allow pop-ups for this site, then try again.'); return; }
    if (err?.code !== 'DOC_NOT_FOUND') { handleError(err); return; }
    state.deletedDocs.add(docId);
    notify('That file was deleted. Its citations now show "file deleted".');
    if (state.chat) refreshTurns();
  }
}

document.addEventListener('click', (e) => {
  const src = e.target.closest('[data-src-url]');
  if (src) {
    e.preventDefault();
    api.openExternal(src.dataset.srcUrl).catch(handleError);
    return;
  }
  const link = e.target.closest('[data-doc]');
  if (!link) return;
  e.preventDefault();
  openDoc(link.dataset.doc, Number(link.dataset.page) || null);
});
