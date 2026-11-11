// Checks the docIds a chat attaches (docs/contract.md "Routes", POST/PATCH /v1/chats): at most the
// configured MAX_ATTACHED_DOCS, each an existing ready doc of the owner. Shared by create-chat and
// update-chat. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Principal } from '../../domain/model.js';
import type { DocStore } from '../../ports/doc-store.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** The bound use case, as create-chat and update-chat receive it. */
export type ResolveDocs = (
  principal: Principal,
  docIds: readonly string[],
) => Promise<Result<string[], AppError>>;

/**
 * Builds the resolve-attached-docs use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The doc store and the configured cap.
 * @returns A function from the principal and requested ids to the de-duplicated ids in request
 *   order, or `VALIDATION_FAILED` (over the cap) or `DOC_NOT_FOUND` (unknown or not ready).
 */
export const resolveAttachedDocs =
  (deps: { readonly docs: DocStore; readonly maxAttachedDocs: number }): ResolveDocs =>
  async (principal, docIds) => {
    const unique = [...new Set(docIds)];
    if (unique.length > deps.maxAttachedDocs)
      return err(
        appError('VALIDATION_FAILED', `attach at most ${String(deps.maxAttachedDocs)} docs`),
      );
    if (unique.length === 0) return ok([]);
    const found = await deps.docs.getMany(principal.uid, unique);
    if (!found.ok) return found;
    const ready = new Set(found.value.filter((d) => d.status === 'ready').map((d) => d.id));
    return unique.every((id) => ready.has(id))
      ? ok(unique)
      : err(appError('DOC_NOT_FOUND', 'a doc is missing or not uploaded'));
  };
