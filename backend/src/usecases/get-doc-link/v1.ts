// GET /v1/docs/{id}/link use case (docs/contract.md "Routes", "v2 Large PDFs"): a 300 s presigned
// GET of a ready doc's PDF, or of one of its parts with `part`. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { docObjectKey, docPartKey } from '../../domain/keys/index.js';
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
 * @returns A function from the principal, doc id and optional part number to the link, or
 *   `DOC_NOT_FOUND` (missing, not ready, or no such part).
 */
export const getDocLink =
  (deps: GetDocLinkDeps) =>
  async (
    principal: Principal,
    docId: string,
    part: number | undefined,
  ): Promise<Result<PresignedUrl, AppError>> => {
    const found = await deps.docs.get(principal.uid, docId);
    if (!found.ok) return found;
    const doc = found.value.value;
    if (doc.status !== 'ready') return err(appError('DOC_NOT_FOUND', 'not uploaded'));
    if (part === undefined)
      return deps.files.presignGet(docObjectKey(principal.uid, docId), DOC_LINK_TTL_SECONDS);
    if (!doc.parts.some((p) => p.n === part)) return err(appError('DOC_NOT_FOUND', 'no such part'));
    return deps.files.presignGet(docPartKey(principal.uid, docId, part), DOC_LINK_TTL_SECONDS);
  };
