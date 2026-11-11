// DocStore port: library documents. Contract: docs/contract.md "Storage" (Doc item).
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { Doc, DocPart, DocStatus, Page, Versioned } from '../domain/model.js';

/** Fields a doc update may change. */
export interface DocPatch {
  readonly title?: string;
  readonly status?: Exclude<DocStatus, 'pending'>;
  readonly pageCount?: number;
  readonly parts?: readonly DocPart[];
}

/** Stores an owner's documents. Every method is scoped by `uid`. */
export interface DocStore {
  /** Creates a doc; fails with `INTERNAL` when the id already exists. */
  create(uid: string, doc: Doc, objectKey: string): Promise<Result<void, AppError>>;
  /** Reads one doc; `DOC_NOT_FOUND` when absent. */
  get(uid: string, docId: string): Promise<Result<Versioned<Doc>, AppError>>;
  /** Reads several docs; ids that do not exist are simply missing from the result. */
  getMany(uid: string, docIds: readonly string[]): Promise<Result<Doc[], AppError>>;
  /** Lists docs newest first (id descending; ids are time-ordered); `VALIDATION_FAILED` for a foreign cursor. */
  list(
    uid: string,
    cursor: string | undefined,
    limit: number,
  ): Promise<Result<Page<Doc>, AppError>>;
  /** Applies a patch if the stored version still equals `version`; else `VERSION_CONFLICT`. */
  update(
    uid: string,
    docId: string,
    patch: DocPatch,
    version: number,
  ): Promise<Result<Doc, AppError>>;
  /** Deletes the doc item; deleting an absent doc is not an error. */
  remove(uid: string, docId: string): Promise<Result<void, AppError>>;
}
