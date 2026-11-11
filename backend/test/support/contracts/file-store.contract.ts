// The FileStore contract suite: run against the in-memory fake and the S3 adapter (over a
// versioned in-memory S3 client). `make` returns the store and how to upload an object.
import { describe, expect, it } from 'vitest';
import type { FileStore } from '../../../src/ports/file-store.js';

/** A store plus the out-of-band upload (the browser's presigned PUT) and a re-upload. */
export interface FileStoreWorld {
  readonly store: FileStore;
  readonly upload: (key: string, bytes: string) => void;
}

/** Registers the suite for a world factory (each call returns an empty bucket). */
export const fileStoreContract = (name: string, make: () => FileStoreWorld) => {
  describe(`FileStore contract: ${name}`, () => {
    it('reports size and first bytes, null size for a missing object', async () => {
      const { store, upload } = make();
      upload('docs/u/a.pdf', '%PDF-1.7');
      expect(await store.sizeOf('docs/u/a.pdf')).toEqual({ ok: true, value: 8 });
      expect(await store.sizeOf('docs/u/b.pdf')).toEqual({ ok: true, value: null });
      expect(await store.firstBytes('docs/u/a.pdf', 5)).toEqual({
        ok: true,
        value: Buffer.from('%PDF-'),
      });
    });

    it('purges every object (and every version) under a prefix, and only there; repeatable', async () => {
      const { store, upload } = make();
      upload('docs/u/a.pdf', 'one');
      upload('docs/u/a.pdf', 'two');
      upload('docs/u/a/part-1.pdf', 'p1');
      upload('docs/u/b.pdf', 'keep');
      const first = await store.purgePrefix('docs/u/a.pdf');
      const parts = await store.purgePrefix('docs/u/a/');
      expect(first.ok && first.value).toBeGreaterThanOrEqual(1);
      expect(parts.ok && parts.value).toBeGreaterThanOrEqual(1);
      expect(await store.sizeOf('docs/u/a.pdf')).toEqual({ ok: true, value: null });
      expect(await store.sizeOf('docs/u/a/part-1.pdf')).toEqual({ ok: true, value: null });
      expect(await store.sizeOf('docs/u/b.pdf')).toEqual({ ok: true, value: 4 });
      expect(await store.purgePrefix('docs/u/a.pdf')).toEqual({ ok: true, value: 0 });
    });

    it('writes and reads back a whole object; reading a missing one is UPLOAD_INVALID', async () => {
      const { store } = make();
      expect((await store.put('docs/u/a/part-1.pdf', Buffer.from('%PDF-1'))).ok).toBe(true);
      expect(await store.read('docs/u/a/part-1.pdf')).toEqual({
        ok: true,
        value: Buffer.from('%PDF-1'),
      });
      expect(await store.read('docs/u/none.pdf')).toMatchObject({
        error: { code: 'UPLOAD_INVALID' },
      });
    });
  });
};
