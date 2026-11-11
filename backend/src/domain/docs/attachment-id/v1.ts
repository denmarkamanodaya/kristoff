// Attachment ids in a chat's `docIds` (docs/contract.md "v2 Large PDFs"): `<docId>` (the whole doc)
// or `<docId>:p<n>` (one part, n 1..999). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** A parsed attachment id. `part` is null for the whole document. */
export interface AttachmentRef {
  readonly docId: string;
  readonly part: number | null;
}

const PATTERN = /^([a-f0-9]{16})(?::p([0-9]{1,3}))?$/;

/**
 * Parses an attachment id.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param id - An entry of `docIds`.
 * @returns The doc id and part number, or `undefined` when the id is malformed or the part is 0.
 */
export const parseAttachmentId = (id: string): AttachmentRef | undefined => {
  const match = PATTERN.exec(id);
  if (match?.[1] === undefined) return undefined;
  const part = match[2] === undefined ? null : Number(match[2]);
  return part === 0 ? undefined : { docId: match[1], part };
};

/**
 * Whether an attachment id points at a document (whole or any of its parts).
 * @param id - An entry of `docIds`.
 * @param docId - 16-hex document id.
 * @returns True for `<docId>` and `<docId>:p<n>`.
 */
export const isAttachmentOf = (id: string, docId: string): boolean =>
  parseAttachmentId(id)?.docId === docId;
