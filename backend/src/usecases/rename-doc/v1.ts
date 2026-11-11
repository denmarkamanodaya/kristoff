// PATCH /v1/docs/{id} use case (docs/contract.md "Routes"): renames a doc (title 1..200, trimmed).
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Doc, Principal } from '../../domain/model.js';
import type { DocStore } from '../../ports/doc-store.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, type Result } from '../../shared/result/result.js';

/**
 * Builds the rename-doc use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The doc store.
 * @returns A function from the principal, doc id and title to the renamed doc, or
 *   `VALIDATION_FAILED` (blank title), `DOC_NOT_FOUND` and `VERSION_CONFLICT`.
 */
export const renameDoc =
  (deps: { readonly docs: DocStore }) =>
  async (principal: Principal, docId: string, title: string): Promise<Result<Doc, AppError>> => {
    const trimmed = title.trim();
    if (trimmed === '') return err(appError('VALIDATION_FAILED', 'title is blank'));
    const found = await deps.docs.get(principal.uid, docId);
    if (!found.ok) return found;
    return deps.docs.update(principal.uid, docId, { title: trimmed }, found.value.version);
  };
