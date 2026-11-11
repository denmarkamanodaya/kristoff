// The UsageMeter contract suite: run against the in-memory fake and the DynamoDB adapter.
import { describe, expect, it } from 'vitest';
import type { UsageMeter } from '../../../src/ports/usage-meter.js';

/** Registers the suite for a meter factory (each call returns an empty meter). */
export const usageMeterContract = (name: string, make: () => Promise<UsageMeter>) => {
  describe(`UsageMeter contract: ${name}`, () => {
    it('admits up to the cap per day, then refuses; a new day starts again', async () => {
      const meter = await make();
      expect((await meter.admitQuestion('u1', '2026-11-12', 2)).ok).toBe(true);
      expect((await meter.admitQuestion('u1', '2026-11-12', 2)).ok).toBe(true);
      expect(await meter.admitQuestion('u1', '2026-11-12', 2)).toMatchObject({
        error: { code: 'DAILY_CAP_REACHED' },
      });
      expect((await meter.admitQuestion('u1', '2026-11-13', 2)).ok).toBe(true);
    });

    it('adds tokens to the day and the total; reads zeros when nothing was counted', async () => {
      const meter = await make();
      const zero = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
      expect(await meter.read('u1', '2026-11-12')).toEqual({
        ok: true,
        value: { today: zero, total: zero },
      });
      const one = { input: 1, output: 2, cacheRead: 3, cacheWrite: 4 };
      await meter.addTokens('u1', '2026-11-11', one);
      await meter.addTokens('u1', '2026-11-12', one);
      expect(await meter.read('u1', '2026-11-12')).toEqual({
        ok: true,
        value: { today: one, total: { input: 2, output: 4, cacheRead: 6, cacheWrite: 8 } },
      });
    });
  });
};
