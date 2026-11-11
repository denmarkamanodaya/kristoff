import { describe, expect, it } from 'vitest';
import { mapLimit } from './map-limit.js';

describe('mapLimit', () => {
  it('keeps input order and never runs more than the limit at once', async () => {
    let running = 0;
    let peak = 0;
    const out = await mapLimit([5, 1, 4, 2, 3], 2, async (x) => {
      running += 1;
      peak = Math.max(peak, running);
      await new Promise((resolve) => {
        setImmediate(resolve);
      });
      running -= 1;
      return x * 10;
    });

    expect(out).toEqual([50, 10, 40, 20, 30]);
    expect(peak).toBe(2);
  });

  it('handles no items and refuses a limit below 1', async () => {
    expect(await mapLimit([], 3, () => Promise.resolve(1))).toEqual([]);
    await expect(mapLimit([1], 0, () => Promise.resolve(1))).rejects.toThrow(RangeError);
  });
});
