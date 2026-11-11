// FileStore port: PDF objects in the docs bucket. Keys come only from src/domain/keys.
// Contract: docs/contract.md "Routes" (upload, confirm, link) and "Model call".
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';

/** A presigned URL and when it stops working (ISO instant). */
export interface PresignedUrl {
  readonly url: string;
  readonly expiresAt: string;
}

/** Reads and presigns objects in the docs bucket. */
export interface FileStore {
  /** A PUT URL pinned to the key, the PDF content type and the exact size. */
  presignPut(
    key: string,
    size: number,
    ttlSeconds: number,
  ): Promise<Result<PresignedUrl, AppError>>;
  /** A GET URL that serves the object as `application/pdf`. */
  presignGet(key: string, ttlSeconds: number): Promise<Result<PresignedUrl, AppError>>;
  /** The object's size in bytes, or `null` when it does not exist. */
  sizeOf(key: string): Promise<Result<number | null, AppError>>;
  /** The object's first `count` bytes (fewer when it is shorter). */
  firstBytes(key: string, count: number): Promise<Result<Uint8Array, AppError>>;
  /**
   * Permanently deletes every version and delete marker of every object whose key starts with
   * `prefix`. Repeatable: a second call deletes nothing.
   * @returns How many versions (and markers) were deleted.
   */
  purgePrefix(prefix: string): Promise<Result<number, AppError>>;
  /** The whole object (docs-split reads up to 200 MiB); `UPLOAD_INVALID` when it is missing. */
  read(key: string): Promise<Result<Uint8Array, AppError>>;
  /** Writes a PDF object (a part produced by docs-split), replacing any previous version. */
  put(key: string, bytes: Uint8Array): Promise<Result<void, AppError>>;
}
