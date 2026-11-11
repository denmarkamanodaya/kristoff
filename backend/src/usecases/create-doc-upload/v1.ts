// POST /v1/docs use case (docs/contract.md "Routes"): records a pending doc and returns a presigned
// PUT pinned to the server-built key, the PDF type and the exact size.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { docObjectKey } from '../../domain/keys/index.js';
import { MAX_DOC_TITLE, PDF_CONTENT_TYPE, UPLOAD_URL_TTL_SECONDS } from '../../domain/limits.js';
import type { Doc, Principal } from '../../domain/model.js';
import type { Clock } from '../../ports/clock.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import type { IdGenerator } from '../../ports/id-generator.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

/** What create-doc-upload needs. */
export interface CreateDocUploadDeps {
  readonly docs: DocStore;
  readonly files: FileStore;
  readonly ids: IdGenerator;
  readonly clock: Clock;
}

/** The upload request: original file name (≤ 255) and size in bytes (1..32 MiB). */
export interface DocUploadInput {
  readonly fileName: string;
  readonly size: number;
}

/** The pending doc and where the browser PUTs the bytes. */
export interface DocUpload {
  readonly doc: Doc;
  readonly uploadUrl: string;
  readonly uploadHeaders: { readonly 'Content-Type': string };
}

/** Title shown for a file whose name is only an extension or blank. */
const UNTITLED = 'Document';

const titleOf = (fileName: string): string =>
  fileName
    .replace(/\.pdf$/i, '')
    .trim()
    .slice(0, MAX_DOC_TITLE) || UNTITLED;

/**
 * Builds the create-doc-upload use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Doc store, file store, id generator and clock.
 * @returns A function from the principal and upload request to the pending doc and its upload
 *   URL; store and presign failures pass through.
 */
export const createDocUpload =
  (deps: CreateDocUploadDeps) =>
  async (principal: Principal, input: DocUploadInput): Promise<Result<DocUpload, AppError>> => {
    const doc: Doc = {
      id: deps.ids.newId(),
      title: titleOf(input.fileName),
      size: input.size,
      status: 'pending',
      createdAt: new Date(deps.clock.now()).toISOString(),
    };
    const key = docObjectKey(principal.uid, doc.id);
    const created = await deps.docs.create(principal.uid, doc, key);
    if (!created.ok) return created;
    const put = await deps.files.presignPut(key, input.size, UPLOAD_URL_TTL_SECONDS);
    if (!put.ok) return put;
    return ok({
      doc,
      uploadUrl: put.value.url,
      uploadHeaders: { 'Content-Type': PDF_CONTENT_TYPE },
    });
  };
