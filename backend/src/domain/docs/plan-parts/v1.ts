// The part-splitting rules for large PDFs (docs/contract.md "v2 Large PDFs"): a document is sent
// whole only when it has ≤ 100 pages and ≤ 30 MiB; otherwise it becomes parts of ≤ 100 pages, and a
// part that comes out larger than 30 MiB is split in half until it fits (a single page that is still
// too large cannot be sent). Pure. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { PART_MAX_BYTES, PART_MAX_PAGES } from '../../limits.js';

/** A 1-based inclusive page range. */
export interface PageRange {
  readonly startPage: number;
  readonly endPage: number;
}

/**
 * Whether a document must be split into parts.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param pageCount - Pages in the PDF (≥ 1).
 * @param size - Bytes of the whole PDF.
 * @returns True when it has more than 100 pages or more than 30 MiB.
 */
export const needsParts = (pageCount: number, size: number): boolean =>
  pageCount > PART_MAX_PAGES || size > PART_MAX_BYTES;

/**
 * The first cut: consecutive ranges of at most 100 pages covering every page.
 * @param pageCount - Pages in the PDF (≥ 1).
 * @returns Ranges in page order.
 */
export const initialRanges = (pageCount: number): PageRange[] =>
  Array.from({ length: Math.ceil(pageCount / PART_MAX_PAGES) }, (_, i) => ({
    startPage: i * PART_MAX_PAGES + 1,
    endPage: Math.min(pageCount, (i + 1) * PART_MAX_PAGES),
  }));

/**
 * Splits a range that came out too large into two halves.
 * @param range - The oversized range.
 * @returns The two halves in order, or `undefined` for a single page (cannot shrink further).
 */
export const halveRange = (range: PageRange): [PageRange, PageRange] | undefined => {
  if (range.startPage === range.endPage) return undefined;
  const mid = Math.floor((range.startPage + range.endPage) / 2);
  return [
    { startPage: range.startPage, endPage: mid },
    { startPage: mid + 1, endPage: range.endPage },
  ];
};

/**
 * Whether a produced part fits the per-document byte limit.
 * @param size - Bytes of the produced part.
 * @returns True when ≤ 30 MiB.
 */
export const partFits = (size: number): boolean => size <= PART_MAX_BYTES;
