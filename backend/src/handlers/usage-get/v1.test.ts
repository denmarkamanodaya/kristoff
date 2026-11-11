import { describe, expect, it } from 'vitest';
import { UsageResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createUsageGetHandler } from './v1.js';

describe('usage-get handler v1', () => {
  it('answers {today, total} for the principal', async () => {
    const zero = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
    const seen: unknown[] = [];
    const handler = createUsageGetHandler({
      http: httpDeps(),
      getUsage: (p) => (seen.push(p), Promise.resolve(ok({ today: zero, total: zero }))),
    });

    const res = await handler(httpEvent({ routeKey: 'GET /v1/usage' }));

    expect([res.statusCode, UsageResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { today: zero, total: zero },
      [{ uid: 'kristoff' }],
    ]);
  });
});
