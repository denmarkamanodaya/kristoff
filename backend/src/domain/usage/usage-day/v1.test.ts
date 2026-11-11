import { describe, expect, it } from 'vitest';
import { secondsUntilNextUsageDay, usageDay } from './v1.js';

describe('usage-day v1', () => {
  it('names the UTC day, so 23:59 UTC and 00:00 UTC fall on different days', () => {
    expect(usageDay(Date.UTC(2026, 10, 12, 23, 59, 59, 999))).toBe('2026-11-12');
    expect(usageDay(Date.UTC(2026, 10, 13, 0, 0, 0, 0))).toBe('2026-11-13');
  });

  it('handles a leap day', () => {
    expect(usageDay(Date.UTC(2028, 1, 29, 12))).toBe('2028-02-29');
  });

  it('counts the seconds to the next UTC midnight, never less than 1', () => {
    expect(secondsUntilNextUsageDay(Date.UTC(2026, 10, 12, 23, 0, 0))).toBe(3600);
    expect(secondsUntilNextUsageDay(Date.UTC(2026, 10, 12, 0, 0, 0))).toBe(86_400);
    expect(secondsUntilNextUsageDay(Date.UTC(2026, 10, 12, 23, 59, 59, 999))).toBe(1);
  });
});
