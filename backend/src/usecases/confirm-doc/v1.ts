// POST /v1/docs/{id}/confirm use case (docs/contract.md "Routes"): the object must exist, match
// the declared size and start with `%PDF-`; then the doc becomes `ready`. Idempotent for a doc
// that is already ready. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { docObjectKey } from '../../domain/keys/index.js';
import { PDF_MAGIC } from '../../domain/limits.js';
import type { Doc, Principal } from '../../domain/model.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** What confirm-doc needs. */
export interface ConfirmDocDeps {
  readonly docs: DocStore;
  readonly files: FileStore;
}

const checkObject = async (
  files: FileStore,
  key: string,
  size: number,
): Promise<Result<void, AppError>> => {
  const stored = await files.sizeOf(key);
  if (!stored.ok) return stored;
  if (stored.value === null) return err(appError('UPLOAD_INVALID', 'no file was uploaded'));
  if (stored.value !== size) return err(appError('UPLOAD_INVALID', 'size does not match'));
  const head = await files.firstBytes(key, PDF_MAGIC.length);
  if (!head.ok) return head;
  const isPdf = Buffer.from(head.value).toString('latin1') === PDF_MAGIC;
  return isPdf ? ok(undefined) : err(appError('UPLOAD_INVALID', 'the file is not a PDF'));
};

/**
 * Builds the confirm-doc use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Doc store and file store.
 * @returns A function from the principal and doc id to the ready doc, or `DOC_NOT_FOUND`,
 *   `UPLOAD_INVALID`, `VERSION_CONFLICT` and store failures.
 */
export const confirmDoc =
  (deps: ConfirmDocDeps) =>
  async (principal: Principal, docId: string): Promise<Result<Doc, AppError>> => {
    const found = await deps.docs.get(principal.uid, docId);
    if (!found.ok) return found;
    const { value: doc, version } = found.value;
    if (doc.status === 'ready') return ok(doc);
    const checked = await checkObject(deps.files, docObjectKey(principal.uid, docId), doc.size);
    if (!checked.ok) return checked;
    return deps.docs.update(principal.uid, docId, { status: 'ready' }, version);
  };
