// In-memory FileStore: `put` simulates the browser's presigned upload.
import type { FileStore } from '../../../src/ports/file-store.js';
import { ok } from '../../../src/shared/result/result.js';

/** The fake plus `objects` and the presign calls it saw. */
export const memoryFileStore = (now: () => number = () => 0) => {
  const objects = new Map<string, Uint8Array>();
  const presigned: { method: 'PUT' | 'GET'; key: string; ttl: number; size?: number }[] = [];
  const expiry = (ttl: number) => new Date(now() + ttl * 1000).toISOString();
  const store: FileStore = {
    presignPut: (key, size, ttl) => {
      presigned.push({ method: 'PUT', key, ttl, size });
      return Promise.resolve(ok({ url: `https://put.example/${key}`, expiresAt: expiry(ttl) }));
    },
    presignGet: (key, ttl) => {
      presigned.push({ method: 'GET', key, ttl });
      return Promise.resolve(ok({ url: `https://get.example/${key}`, expiresAt: expiry(ttl) }));
    },
    sizeOf: (key) => Promise.resolve(ok(objects.get(key)?.length ?? null)),
    firstBytes: (key, count) =>
      Promise.resolve(ok((objects.get(key) ?? new Uint8Array()).slice(0, count))),
  };
  return {
    store,
    objects,
    presigned,
    put: (key: string, bytes: string | Uint8Array) =>
      objects.set(key, typeof bytes === 'string' ? Buffer.from(bytes) : bytes),
  };
};
