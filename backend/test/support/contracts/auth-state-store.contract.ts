// The AuthStateStore contract suite: run against the in-memory fake and the DynamoDB adapter.
import { describe, expect, it } from 'vitest';
import type { AuthStateStore } from '../../../src/ports/auth-state-store.js';

/** Registers the suite for a store factory (each call returns an empty store). */
export const authStateStoreContract = (name: string, make: () => Promise<AuthStateStore>) => {
  describe(`AuthStateStore contract: ${name}`, () => {
    it('starts at the initial state with version 0', async () => {
      const store = await make();
      expect(await store.get('u1')).toEqual({
        ok: true,
        value: { value: { lastStep: 0, failCount: 0, lockedUntil: 0 }, version: 0 },
      });
    });

    it('accepts a higher step once and refuses the same or a lower step (replay)', async () => {
      const store = await make();
      expect((await store.acceptStep('u1', 100)).ok).toBe(true);
      expect(await store.acceptStep('u1', 100)).toMatchObject({ error: { code: 'OTP_REPLAYED' } });
      expect(await store.acceptStep('u1', 99)).toMatchObject({ error: { code: 'OTP_REPLAYED' } });
      expect((await store.acceptStep('u1', 101)).ok).toBe(true);
    });

    it('records failures at the read version only, and a success clears them', async () => {
      const store = await make();
      expect(
        (await store.recordFailure('u1', { lastStep: 0, failCount: 1, lockedUntil: 0 }, 0)).ok,
      ).toBe(true);
      expect(
        await store.recordFailure('u1', { lastStep: 0, failCount: 1, lockedUntil: 0 }, 0),
      ).toMatchObject({ error: { code: 'VERSION_CONFLICT' } });
      expect(await store.get('u1')).toMatchObject({
        value: { value: { failCount: 1 }, version: 1 },
      });
      await store.acceptStep('u1', 5);
      expect(await store.get('u1')).toMatchObject({
        value: { value: { lastStep: 5, failCount: 0, lockedUntil: 0 }, version: 2 },
      });
    });
  });
};
