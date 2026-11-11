import { describe, expect, it } from 'vitest';
import { LinkResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsLinkHandler } from './v1.js';

const ROUTE = 'GET /v1/docs/{id}/link';
const LINK = { url: 'https://get.example/x', expiresAt: '2026-11-12T00:05:00.000Z' };

describe('docs-link handler v1', () => {
  it('answers {url, expiresAt} for the path id', async () => {
    const seen: unknown[] = [];
    const handler = createDocsLinkHandler({
      http: httpDeps(),
      getDocLink: (p, id, part) => (seen.push([p, id, part]), Promise.resolve(ok(LINK))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }));
    await handler(
      httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' }, query: { part: '12' } }),
    );

    expect([res.statusCode, LinkResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      LINK,
      [
        [{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa', undefined],
        [{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa', 12],
      ],
    ]);
  });

  it('answers 404 DOC_NOT_FOUND', async () => {
    const handler = createDocsLinkHandler({
      http: httpDeps(),
      getDocLink: () => Promise.resolve(err(appError('DOC_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }))).statusCode,
    ).toBe(404);
  });

  it.each([['0'], ['1000'], ['x']])('answers 400 for part=%s', async (part) => {
    const handler = createDocsLinkHandler({
      http: httpDeps(),
      getDocLink: () => Promise.reject(new Error('called')),
    });

    expect(
      (
        await handler(
          httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' }, query: { part } }),
        )
      ).statusCode,
    ).toBe(400);
  });
});
