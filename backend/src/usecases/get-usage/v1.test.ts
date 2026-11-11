import { describe, expect, it } from 'vitest';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { memoryUsageMeter } from '../../../test/support/fakes/memory-usage-meter.js';
import { getUsage } from './v1.js';

describe('get-usage v1', () => {
  it("reads today's UTC counters and the total", async () => {
    const usage = memoryUsageMeter();
    const one = { input: 1, output: 2, cacheRead: 3, cacheWrite: 4 };
    await usage.meter.addTokens('kristoff', '2026-11-11', one);
    await usage.meter.addTokens('kristoff', '2026-11-12', one);

    const result = await getUsage({ meter: usage.meter, clock: fakeClock() })({ uid: 'kristoff' });

    expect(result).toEqual({
      ok: true,
      value: { today: one, total: { input: 2, output: 4, cacheRead: 6, cacheWrite: 8 } },
    });
  });
});
