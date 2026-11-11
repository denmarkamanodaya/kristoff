// The DocStore contract suite (core/architecture-hexagonal.md "Port contract suite"): run against
// the in-memory fake and the DynamoDB adapter, so the fake is trusted.
import { describe, expect, it } from 'vitest';
import type { DocStore } from '../../../src/ports/doc-store.js';
import { aDoc } from '../builders.js';

/** Registers the suite for a store factory (each call returns an empty store). */
export const docStoreContract = (name: string, make: () => Promise<DocStore>) => {
  describe(`DocStore contract: ${name}`, () => {
    it('creates once and reads back with version 1; a second create fails', async () => {
      const store = await make();
      expect(await store.create('u1', aDoc({ status: 'pending' }), 'k')).toEqual({
        ok: true,
        value: undefined,
      });
      expect(await store.get('u1', aDoc().id)).toEqual({
        ok: true,
        value: { value: aDoc({ status: 'pending' }), version: 1 },
      });
      expect((await store.create('u1', aDoc(), 'k')).ok).toBe(false);
    });

    it('scopes reads by owner', async () => {
      const store = await make();
      await store.create('u1', aDoc(), 'k');
      expect(await store.get('u2', aDoc().id)).toMatchObject({
        ok: false,
        error: { code: 'DOC_NOT_FOUND' },
      });
      expect(await store.getMany('u2', [aDoc().id])).toEqual({ ok: true, value: [] });
    });

    it('updates only at the read version and bumps it', async () => {
      const store = await make();
      await store.create('u1', aDoc({ status: 'pending' }), 'k');
      expect(await store.update('u1', aDoc().id, { status: 'ready' }, 1)).toEqual({
        ok: true,
        value: aDoc(),
      });
      expect(await store.update('u1', aDoc().id, { title: 'x' }, 1)).toMatchObject({
        error: { code: 'VERSION_CONFLICT' },
      });
      expect(await store.get('u1', aDoc().id)).toMatchObject({ value: { version: 2 } });
    });

    it('pages a list newest first (id descending) with an opaque cursor and rejects a foreign cursor', async () => {
      const store = await make();
      for (const id of ['0000000000000001', '0000000000000002', '0000000000000003'])
        await store.create('u1', aDoc({ id }), 'k');
      const first = await store.list('u1', undefined, 2);
      expect(first.ok && first.value.items.map((d) => d.id)).toEqual([
        '0000000000000003',
        '0000000000000002',
      ]);
      const cursor = first.ok ? first.value.nextCursor : null;
      const second = await store.list('u1', cursor ?? undefined, 2);
      expect(second.ok && second.value.items.map((d) => d.id)).toEqual(['0000000000000001']);
      expect(await store.list('u1', 'not-a-cursor!', 2)).toMatchObject({
        error: { code: 'VALIDATION_FAILED' },
      });
    });
  });
};
