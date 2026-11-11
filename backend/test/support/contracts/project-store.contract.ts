// The ProjectStore contract suite: run against the in-memory fake and the DynamoDB adapter.
import { describe, expect, it } from 'vitest';
import type { ProjectStore } from '../../../src/ports/project-store.js';

const P = { id: 'pppppppppppppppp', name: 'Torts', createdAt: 'a', updatedAt: 'a' };

/** Registers the suite for a store factory (each call returns an empty store). */
export const projectStoreContract = (name: string, make: () => Promise<ProjectStore>) => {
  describe(`ProjectStore contract: ${name}`, () => {
    it('creates up to the cap per owner, then PROJECT_LIMIT_REACHED; delete frees a slot', async () => {
      const store = await make();
      expect((await store.create('u1', P, 2)).ok).toBe(true);
      expect((await store.create('u1', { ...P, id: '0000000000000002' }, 2)).ok).toBe(true);
      expect(await store.create('u1', { ...P, id: '0000000000000003' }, 2)).toMatchObject({
        error: { code: 'PROJECT_LIMIT_REACHED' },
      });
      expect((await store.create('u2', P, 2)).ok).toBe(true);
      expect((await store.delete('u1', P.id)).ok).toBe(true);
      expect((await store.create('u1', { ...P, id: '0000000000000003' }, 2)).ok).toBe(true);
    });

    it('reads, lists and renames under the owner only, version-locked', async () => {
      const store = await make();
      await store.create('u1', P, 50);
      expect(await store.get('u1', P.id)).toEqual({ ok: true, value: { value: P, version: 1 } });
      expect(await store.get('u2', P.id)).toMatchObject({ error: { code: 'PROJECT_NOT_FOUND' } });
      expect(await store.list('u2')).toEqual({ ok: true, value: [] });
      expect(await store.rename('u1', P.id, { name: 'New', updatedAt: 'b', version: 1 })).toEqual({
        ok: true,
        value: { ...P, name: 'New', updatedAt: 'b' },
      });
      expect(
        await store.rename('u1', P.id, { name: 'X', updatedAt: 'c', version: 1 }),
      ).toMatchObject({ error: { code: 'VERSION_CONFLICT' } });
      expect(await store.list('u1')).toEqual({
        ok: true,
        value: [{ ...P, name: 'New', updatedAt: 'b' }],
      });
    });

    it('answers PROJECT_NOT_FOUND when deleting an absent project', async () => {
      const store = await make();
      expect(await store.delete('u1', P.id)).toMatchObject({
        error: { code: 'PROJECT_NOT_FOUND' },
      });
    });
  });
};
