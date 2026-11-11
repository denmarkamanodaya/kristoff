import { describe, expect, it } from 'vitest';
import { sumUsage } from './v1.js';

describe('sum-usage v1', () => {
  it('adds field by field and treats null as zero', () => {
    const a = { input: 1, output: 2, cacheRead: 3, cacheWrite: 4 };

    expect(sumUsage(a, { input: 10, output: 20, cacheRead: 30, cacheWrite: 40 })).toEqual({
      input: 11,
      output: 22,
      cacheRead: 33,
      cacheWrite: 44,
    });
    expect(sumUsage(a, null)).toEqual(a);
  });
});
