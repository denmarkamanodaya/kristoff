// What an attachment id points at (docs/contract.md "v2 Large PDFs"): a whole doc is attachable only
// when it is ready and has no parts; a part id only when the doc is ready and has that part. One
// rule for the chat docIds check and for the ask Lambda. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Doc, DocPart } from '../../model.js';
import type { AttachmentRef } from '../attachment-id/index.js';

/** The doc as one model document, or one of its parts. */
export type AttachmentTarget =
  | { readonly kind: 'whole'; readonly doc: Doc }
  | { readonly kind: 'part'; readonly doc: Doc; readonly part: DocPart };

/**
 * Resolves a parsed attachment id against its doc.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param ref - The parsed attachment id.
 * @param doc - The doc it names (same id), or undefined when it does not exist.
 * @returns The target, or `undefined` when the doc is missing, not ready, attached whole although
 *   it has parts, or the part does not exist.
 */
export const attachmentTarget = (
  ref: AttachmentRef,
  doc: Doc | undefined,
): AttachmentTarget | undefined => {
  if (doc?.status !== 'ready' || doc.id !== ref.docId) return undefined;
  if (ref.part === null) return doc.parts.length === 0 ? { kind: 'whole', doc } : undefined;
  const part = doc.parts.find((p) => p.n === ref.part);
  return part === undefined ? undefined : { kind: 'part', doc, part };
};
