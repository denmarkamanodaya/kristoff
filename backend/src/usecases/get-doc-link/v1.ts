// GET /v1/docs/{id}/link use case (docs/contract.md "Routes"): a 300 s presigned GET of a ready
// doc's PDF. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { docObjectKey } from '../../domain/keys/index.js';
import { DOC_LINK_TTL_SECONDS } from '../../domain/limits.js';
import type { Principal } from '../../domain/model.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore, PresignedUrl } from '../../ports/file-store.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, type Result } from '../../shared/result/result.js';

/** What get-doc-link needs. */
export interface GetDocLinkDeps {
  readonly docs: DocStore;
  readonly files: FileStore;
}

/**
 * Builds the get-doc-link use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Doc store and file store.
 * @returns A function from the principal and doc id to the link, or `DOC_NOT_FOUND` (also for a
 *   doc whose upload was never confirmed).
 */
export const getDocLink =
  (deps: GetDocLinkDeps) =>
  async (principal: Principal, docId: string): Promise<Result<PresignedUrl, AppError>> => {
    const found = await deps.docs.get(principal.uid, docId);
    if (!found.ok) return found;
    if (found.value.value.status !== 'ready') return err(appError('DOC_NOT_FOUND', 'not uploaded'));
    return deps.files.presignGet(docObjectKey(principal.uid, docId), DOC_LINK_TTL_SECONDS);
  };
