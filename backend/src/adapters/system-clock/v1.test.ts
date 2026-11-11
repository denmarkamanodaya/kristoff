import { describe, expect, it } from 'vitest';
import { systemClock } from './v1.js';

describe('system-clock v1', () => {
  it('reads the host clock in epoch milliseconds', () => {
    const before = Date.now();

    const now = systemClock.now();

    expect(now).toBeGreaterThanOrEqual(before);
    expect(now).toBeLessThanOrEqual(Date.now());
  });
});
