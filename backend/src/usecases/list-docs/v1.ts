// GET /v1/docs use case (docs/contract.md "Routes"): one bounded page of the owner's library.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { DOCS_PAGE_SIZE } from '../../domain/limits.js';
import type { Doc, Page, Principal } from '../../domain/model.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import type { Result } from '../../shared/result/result.js';

/**
 * Builds the list-docs use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The doc store.
 * @returns A function from the principal and opaque cursor to one page of up to 100 docs, or
 *   `VALIDATION_FAILED` for a foreign cursor.
 */
export const listDocs =
  (deps: { readonly docs: DocStore }) =>
  (principal: Principal, cursor: string | undefined): Promise<Result<Page<Doc>, AppError>> =>
    deps.docs.list(principal.uid, cursor, DOCS_PAGE_SIZE);
