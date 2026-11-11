// S3FileStore: the FileStore port on the docs bucket (docs/contract.md "Routes", "Storage").
// Presigned PUT signs content-type and content-length, so the browser can only upload a PDF of
// the declared size to the server-built key. purgePrefix removes every version (bucket is versioned). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import {
  DeleteObjectCommand,
  GetObjectCommand,
  HeadObjectCommand,
  ListObjectVersionsCommand,
  PutObjectCommand,
  type S3Client,
} from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { PDF_CONTENT_TYPE } from '../../domain/limits.js';
import type { FileStore } from '../../ports/file-store.js';
import { mapLimit } from '../../shared/map-limit/map-limit.js';
import { upstreamError } from '../../shared/errors/vendor-error.js';
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
    return err(upstreamError(operation, error));
  }
};

const read =
  ({ client, bucket }: S3Target): FileStore['read'] =>
  async (key) => {
    try {
      const got = await client.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
      return ok(got.Body ? await got.Body.transformToByteArray() : new Uint8Array());
    } catch (error) {
      return err(
        isNotFound(error)
          ? appError('UPLOAD_INVALID', 'the file is missing')
          : upstreamError('s3.GetObject', error),
      );
    }
  };

const put =
  ({ client, bucket }: S3Target): FileStore['put'] =>
  async (key, bytes) => {
    const sent = await attempt('s3.PutObject', () =>
      client.send(
        new PutObjectCommand({
          Bucket: bucket,
          Key: key,
          Body: bytes,
          ContentType: PDF_CONTENT_TYPE,
        }),
      ),
    );
    return sent.ok ? ok(undefined) : sent;
  };

const sizeOf =
  ({ client, bucket }: S3Target): FileStore['sizeOf'] =>
  async (key) => {
    try {
      const head = await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
      return ok(head.ContentLength ?? 0);
    } catch (error) {
      return isNotFound(error) ? ok(null) : err(upstreamError('s3.HeadObject', error));
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

/** Most DeleteObject calls in flight at once while purging. */
const PURGE_CONCURRENCY = 8;

/** Where a ListObjectVersions page starts (both absent = the first page). */
interface VersionMarker {
  readonly key?: string | undefined;
  readonly versionId?: string | undefined;
}

/** One object version (or delete marker) to remove. */
interface VersionRef {
  readonly Key: string;
  readonly VersionId: string;
}

/** One page of versions and delete markers under a prefix, plus where the next page starts. */
const listVersionPage = async (
  { client, bucket }: S3Target,
  prefix: string,
  marker: VersionMarker,
): Promise<Result<{ refs: VersionRef[]; next?: VersionMarker }, AppError>> =>
  attempt('s3.ListObjectVersions', async () => {
    const page = await client.send(
      new ListObjectVersionsCommand({
        Bucket: bucket,
        Prefix: prefix,
        ...(marker.key === undefined ? {} : { KeyMarker: marker.key }),
        ...(marker.versionId === undefined ? {} : { VersionIdMarker: marker.versionId }),
      }),
    );
    const refs = [...(page.Versions ?? []), ...(page.DeleteMarkers ?? [])].flatMap((v) =>
      v.Key !== undefined && v.VersionId !== undefined
        ? [{ Key: v.Key, VersionId: v.VersionId }]
        : [],
    );
    const next =
      page.IsTruncated === true
        ? { key: page.NextKeyMarker, versionId: page.NextVersionIdMarker }
        : undefined;
    return next === undefined ? { refs } : { refs, next };
  });

const deleteVersions = async (
  { client, bucket }: S3Target,
  refs: readonly VersionRef[],
): Promise<Result<void, AppError>> => {
  const results = await mapLimit(refs, PURGE_CONCURRENCY, (ref) =>
    attempt('s3.DeleteObject', () =>
      client.send(
        new DeleteObjectCommand({ Bucket: bucket, Key: ref.Key, VersionId: ref.VersionId }),
      ),
    ),
  );
  return results.find((r) => !r.ok) ?? ok(undefined);
};

const purgePrefix =
  (target: S3Target): FileStore['purgePrefix'] =>
  async (prefix) => {
    let deleted = 0;
    let marker: VersionMarker | undefined = {};
    while (marker !== undefined) {
      const page = await listVersionPage(target, prefix, marker);
      if (!page.ok) return page;
      const done = await deleteVersions(target, page.value.refs);
      if (!done.ok) return done;
      deleted += page.value.refs.length;
      marker = page.value.next;
    }
    return ok(deleted);
  };

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
    purgePrefix: purgePrefix(target),
    read: read(target),
    put: put(target),
  };
};
