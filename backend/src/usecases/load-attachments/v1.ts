// Resolves a chat's attachment ids (`<docId>` / `<docId>:p<n>`) against the owner's docs in one
// batch read: the shared first step of resolve-attached-docs (all must be usable) and ask-question
// (unusable ones are skipped). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { parseAttachmentId } from '../../domain/docs/attachment-id/index.js';
import {
  attachmentTarget,
  type AttachmentTarget,
} from '../../domain/docs/attachment-target/index.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

/**
 * Resolves attachment ids in order.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param docs - The doc store.
 * @param uid - Owner id from the principal (scopes the read).
 * @param attachmentIds - Entries of a chat's `docIds`.
 * @returns One target per id, in order; `undefined` where the id is malformed, the doc is missing
 *   or not ready, or the part does not exist. Store failures pass through.
 */
export const loadAttachments = async (
  docs: DocStore,
  uid: string,
  attachmentIds: readonly string[],
): Promise<Result<(AttachmentTarget | undefined)[], AppError>> => {
  const refs = attachmentIds.map(parseAttachmentId);
  const named = [...new Set(refs.flatMap((r) => (r ? [r.docId] : [])))];
  if (named.length === 0) return ok(refs.map(() => undefined));
  const found = await docs.getMany(uid, named);
  if (!found.ok) return found;
  const byId = new Map(found.value.map((d) => [d.id, d]));
  return ok(refs.map((r) => (r ? attachmentTarget(r, byId.get(r.docId)) : undefined)));
};
