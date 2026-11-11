// S3FileStore: the FileStore port on the docs bucket (docs/contract.md "Routes", "Storage").
// Presigned PUT signs content-type and content-length, so the browser can only upload a PDF of
// the declared size to the server-built key. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import {
  GetObjectCommand,
  HeadObjectCommand,
  PutObjectCommand,
  type S3Client,
} from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { PDF_CONTENT_TYPE } from '../../domain/limits.js';
import type { FileStore } from '../../ports/file-store.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** What the store needs: the client (checksums `WHEN_REQUIRED`), bucket and time source. */
export interface S3Target {
  readonly client: S3Client;
  readonly bucket: string;
  /** Epoch milliseconds, for `expiresAt`. */
  readonly now: () => number;
  /** Presign with an injectable signer (the real `getSignedUrl` by default). */
  readonly sign?: typeof getSignedUrl;
}

const upstream = (operation: string, error: unknown): AppError =>
  appError('UPSTREAM_UNAVAILABLE', `${operation} failed`, {
    operation,
    'error.type': error instanceof Error ? error.name.slice(0, 64) : 'Unknown',
  });

const isNotFound = (error: unknown): boolean =>
  error instanceof Error &&
  (error.name === 'NotFound' ||
    (error as { $metadata?: { httpStatusCode?: number } }).$metadata?.httpStatusCode === 404);

const attempt = async <T>(
  operation: string,
  run: () => Promise<T>,
): Promise<Result<T, AppError>> => {
  try {
    return ok(await run());
  } catch (error) {
    return err(upstream(operation, error));
  }
};

const sizeOf =
  ({ client, bucket }: S3Target): FileStore['sizeOf'] =>
  async (key) => {
    try {
      const head = await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
      return ok(head.ContentLength ?? 0);
    } catch (error) {
      return isNotFound(error) ? ok(null) : err(upstream('s3.HeadObject', error));
    }
  };

const firstBytes =
  ({ client, bucket }: S3Target): FileStore['firstBytes'] =>
  (key, count) =>
    attempt('s3.GetObject', async () => {
      const range = `bytes=0-${String(count - 1)}`;
      const got = await client.send(
        new GetObjectCommand({ Bucket: bucket, Key: key, Range: range }),
      );
      return got.Body ? await got.Body.transformToByteArray() : new Uint8Array();
    });

/**
 * Builds the S3 file store.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Client, bucket, clock and optional signer.
 * @returns The FileStore.
 */
export const s3FileStore = (target: S3Target): FileStore => {
  const { client, bucket, now, sign = getSignedUrl } = target;
  const expiry = (ttl: number) => new Date(now() + ttl * 1000).toISOString();
  return {
    presignPut: (key, size, ttl) =>
      attempt('s3.PresignPut', async () => {
        const command = new PutObjectCommand({
          Bucket: bucket,
          Key: key,
          ContentType: PDF_CONTENT_TYPE,
          ContentLength: size,
        });
        const signableHeaders = new Set(['content-type']);
        const url = await sign(client, command, { expiresIn: ttl, signableHeaders });
        return { url, expiresAt: expiry(ttl) };
      }),
    presignGet: (key, ttl) =>
      attempt('s3.PresignGet', async () => {
        const command = new GetObjectCommand({
          Bucket: bucket,
          Key: key,
          ResponseContentType: PDF_CONTENT_TYPE,
        });
        return { url: await sign(client, command, { expiresIn: ttl }), expiresAt: expiry(ttl) };
      }),
    sizeOf: sizeOf(target),
    firstBytes: firstBytes(target),
  };
};
