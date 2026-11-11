// A versioned in-memory S3 client for the commands S3FileStore sends (Head, Get with Range, Put,
// ListObjectVersions with markers, DeleteObject by VersionId), so the real adapter can run the
// FileStore contract suite without AWS. `pageSize` forces ListObjectVersions pagination.
import type { S3Client } from '@aws-sdk/client-s3';

interface Version {
  readonly key: string;
  readonly versionId: string;
  readonly body: Uint8Array | null;
}

const notFound = () =>
  Object.assign(new Error('NotFound'), { name: 'NotFound', $metadata: { httpStatusCode: 404 } });

/** The client plus `put` (what the browser's presigned PUT would do) and the version list. */
export const s3MemoryClient = (pageSize = 1000) => {
  const versions: Version[] = [];
  let n = 0;
  const latest = (key: string) => [...versions].reverse().find((v) => v.key === key);
  const put = (key: string, body: string | Uint8Array) =>
    versions.push({
      key,
      versionId: `v${String((n += 1))}`,
      body: typeof body === 'string' ? Buffer.from(body) : body,
    });
  const handlers: Record<string, (input: Record<string, unknown>) => unknown> = {
    HeadObjectCommand: (i) => {
      const v = latest(String(i.Key));
      if (!v?.body) throw notFound();
      return { ContentLength: v.body.length };
    },
    GetObjectCommand: (i) => {
      const v = latest(String(i.Key));
      if (!v?.body) throw notFound();
      const end = Number(/bytes=0-(\d+)/.exec(String(i.Range))?.[1] ?? v.body.length - 1);
      const bytes = v.body.slice(0, end + 1);
      return { Body: { transformToByteArray: () => Promise.resolve(bytes) } };
    },
    PutObjectCommand: (i) => {
      put(String(i.Key), (i.Body as Uint8Array | undefined) ?? new Uint8Array());
      return {};
    },
    ListObjectVersionsCommand: (i) => {
      const all = versions.filter((v) => v.key.startsWith(String(i.Prefix)));
      const start =
        i.VersionIdMarker === undefined
          ? 0
          : all.findIndex((v) => v.versionId === i.VersionIdMarker) + 1;
      const page = all.slice(start, start + pageSize);
      const last = page.at(-1);
      const isTruncated = start + pageSize < all.length;
      return {
        Versions: page.filter((v) => v.body).map((v) => ({ Key: v.key, VersionId: v.versionId })),
        DeleteMarkers: page
          .filter((v) => !v.body)
          .map((v) => ({ Key: v.key, VersionId: v.versionId })),
        IsTruncated: isTruncated,
        ...(isTruncated && last
          ? { NextKeyMarker: last.key, NextVersionIdMarker: last.versionId }
          : {}),
      };
    },
    DeleteObjectCommand: (i) => {
      const at = versions.findIndex((v) => v.key === i.Key && v.versionId === i.VersionId);
      if (at >= 0) versions.splice(at, 1);
      return {};
    },
  };
  const client = {
    send: (command: { constructor: { name: string }; input: Record<string, unknown> }) => {
      const handle = handlers[command.constructor.name];
      if (!handle) return Promise.reject(new Error(`unsupported ${command.constructor.name}`));
      try {
        return Promise.resolve(handle(command.input));
      } catch (error) {
        return Promise.reject(error instanceof Error ? error : new Error(String(error)));
      }
    },
  };
  /** Adds a delete marker (what a plain DeleteObject on a versioned bucket leaves). */
  const mark = (key: string) =>
    versions.push({ key, versionId: `v${String((n += 1))}`, body: null });
  return { client: client as unknown as S3Client, put, mark, versions };
};
