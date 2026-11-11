import { describe, expect, it } from 'vitest';
import { DocsPageResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsListHandler } from './v1.js';

describe('docs-list handler v1', () => {
  it('passes the principal and cursor, and answers the page', async () => {
    const seen: unknown[] = [];
    const handler = createDocsListHandler({
      http: httpDeps(),
      listDocs: (p, cursor) => (
        seen.push([p, cursor]),
        Promise.resolve(ok({ items: [aDoc()], nextCursor: 'n' }))
      ),
    });

    const res = await handler(httpEvent({ routeKey: 'GET /v1/docs', query: { cursor: 'abc' } }));

    expect([res.statusCode, DocsPageResponse.parse(bodyOf(res))]).toEqual([
      200,
      { items: [aDoc()], nextCursor: 'n' },
    ]);
    expect(seen).toEqual([[{ uid: 'kristoff' }, 'abc']]);
  });

  it('answers 400 for an unknown query parameter', async () => {
    const handler = createDocsListHandler({
      http: httpDeps(),
      listDocs: () => Promise.reject(new Error('called')),
    });

    expect(
      (await handler(httpEvent({ routeKey: 'GET /v1/docs', query: { limit: '5' } }))).statusCode,
    ).toBe(400);
  });
});
