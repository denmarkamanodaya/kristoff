import { describe, expect, it } from 'vitest';
import { DocResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsConfirmHandler } from './v1.js';

const ROUTE = 'POST /v1/docs/{id}/confirm';

describe('docs-confirm handler v1', () => {
  it('passes the path id and answers {doc}', async () => {
    const seen: unknown[] = [];
    const handler = createDocsConfirmHandler({
      http: httpDeps(),
      confirmDoc: (p, id) => (seen.push([p, id]), Promise.resolve(ok(aDoc()))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }));

    expect([res.statusCode, DocResponse.parse(bodyOf(res))]).toEqual([200, { doc: aDoc() }]);
    expect(seen).toEqual([[{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa']]);
  });

  it('answers 400 for a malformed id and 422 for UPLOAD_INVALID', async () => {
    const handler = createDocsConfirmHandler({
      http: httpDeps(),
      confirmDoc: () => Promise.resolve(err(appError('UPLOAD_INVALID', 'not a PDF'))),
    });

    expect((await handler(httpEvent({ routeKey: ROUTE, path: { id: 'ZZ' } }))).statusCode).toBe(
      400,
    );
    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }))).statusCode,
    ).toBe(422);
  });
});
