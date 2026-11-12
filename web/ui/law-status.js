// Law status line (docs/contract.md, "Law status line"): a `kind:"law"` citation's quote may start
// with one `[Status]` label, shown as a badge with optional hover text. Pure (no DOM), so
// test/law-status.test.mjs can load it; ui/citations.js renders the badge.

/** One `[Status]` label: letters, spaces and hyphens, starting with a letter, at most 40 chars. */
export const STATUS_LINE = /^\s*\[([A-Za-z][A-Za-z -]{0,39})\]\s*/;

/** Hover text for status labels that need one; other labels show without hover text. */
export const STATUS_HINTS = Object.freeze({
  'As enacted': 'Original text as first enacted; later amendments may change it.',
  'As promulgated': 'Court rule as promulgated; later amendments may change it.',
  'As issued': 'Executive or agency issuance as issued; it may since have been amended or revoked.',
  'As signed': 'Treaty text as signed.',
  'Constitutional records': 'Records of the constitutional convention or commission; useful for interpretation, not law itself.',
  'Court reference': 'Court reference material (benchbook or manual), not law itself.',
});

/**
 * Splits a citation's quote into its status label (law citations only) and the quote without it.
 * @param c - A Citation.
 * @returns `{ status, quote }`; `status` is null when there is no label or the citation is not a law.
 */
export function lawStatus(c) {
  const quote = String(c.quote ?? '');
  const m = c.kind === 'law' ? STATUS_LINE.exec(quote) : null;
  return m ? { status: m[1].trim(), quote: quote.slice(m[0].length) } : { status: null, quote };
}
