import { describe, expect, it } from 'vitest';
import { ConfigResponse } from '../../contracts/shapes.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createConfigGetHandler } from './v1.js';

describe('config-get handler v1', () => {
  it('answers the configured settings', async () => {
    const config = {
      model: 'claude-sonnet-5-5',
      models: [
        { id: 'claude-sonnet-5-5', label: 'Sonnet 5.5' },
        { id: 'claude-opus-5-5', label: 'Opus 5.5' },
      ],
      maxAttachedDocs: 5,
      dailyQuestionCap: 100,
    };
    const handler = createConfigGetHandler({ http: httpDeps(), getConfig: () => config });

    const res = await handler(httpEvent({ routeKey: 'GET /v1/config' }));

    expect([res.statusCode, ConfigResponse.parse(bodyOf(res))]).toEqual([200, config]);
  });

  it('answers 401 without the authorizer context', async () => {
    const handler = createConfigGetHandler({
      http: httpDeps(),
      getConfig: () => ({
        model: 'm',
        models: [{ id: 'm', label: 'm' }],
        maxAttachedDocs: 1,
        dailyQuestionCap: 1,
      }),
    });

    expect(
      (await handler(httpEvent({ routeKey: 'GET /v1/config', sub: undefined }))).statusCode,
    ).toBe(401);
  });
});
