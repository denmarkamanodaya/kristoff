// Maps a model-reported citation to the stored Citation (docs/contract.md "Shapes"), as the
// original server.js did (end page made inclusive), plus v2: a part's cited pages are mapped back to
// original page numbers (`startPage + page − 1`).
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { MAX_DOC_TITLE, MAX_QUOTE } from '../../limits.js';
import type { Citation } from '../../model.js';
import type { AttachedDoc, CitationSource } from '../model-request.js';

/** Title used when neither the model nor the library names the document. */
const FALLBACK_TITLE = 'Document';

/**
 * Numbers a citation and ties it to the attached library document it points at.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param source - The citation as the model reported it.
 * @param docs - Attached docs in request order (the model's `documentIndex` indexes this).
 * @param n - 1-based citation number within the answer.
 * @returns The citation; `docId` null when the index matches no attached doc.
 */
export const mapCitation = (
  source: CitationSource,
  docs: readonly AttachedDoc[],
  n: number,
): Citation => {
  const doc = docs[source.documentIndex];
  const title = doc?.libraryTitle ?? source.documentTitle ?? FALLBACK_TITLE;
  const offset = doc?.pageOffset ?? 0;
  return {
    n,
    docId: doc?.id ?? null,
    docTitle: title.slice(0, MAX_DOC_TITLE),
    page: source.startPage === null ? null : source.startPage + offset,
    endPage: source.endPageExclusive === null ? null : source.endPageExclusive - 1 + offset,
    quote: source.citedText.slice(0, MAX_QUOTE),
  };
};
