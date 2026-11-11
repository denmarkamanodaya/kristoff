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
      getDocLink: (p, id) => (seen.push([p, id]), Promise.resolve(ok(LINK))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }));

    expect([res.statusCode, LinkResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      LINK,
      [[{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa']],
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
});
