import { describe, expect, it } from 'vitest';
import { DocResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsRenameHandler } from './v1.js';

const ROUTE = 'PATCH /v1/docs/{id}';

describe('docs-rename handler v1', () => {
  it('passes id and title, and answers {doc}', async () => {
    const seen: unknown[] = [];
    const handler = createDocsRenameHandler({
      http: httpDeps(),
      renameDoc: (p, id, t) => (seen.push([p, id, t]), Promise.resolve(ok(aDoc({ title: t })))),
    });

    const res = await handler(
      httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' }, body: { title: 'Cruz' } }),
    );

    expect([res.statusCode, DocResponse.parse(bodyOf(res))]).toEqual([
      200,
      { doc: aDoc({ title: 'Cruz' }) },
    ]);
    expect(seen).toEqual([[{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa', 'Cruz']]);
  });

  it('answers 400 for an empty or 201-char title', async () => {
    const handler = createDocsRenameHandler({
      http: httpDeps(),
      renameDoc: () => Promise.reject(new Error('called')),
    });
    const path = { id: 'aaaaaaaaaaaaaaaa' };

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path, body: { title: '' } }))).statusCode,
    ).toBe(400);
    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path, body: { title: 'x'.repeat(201) } })))
        .statusCode,
    ).toBe(400);
  });
});
