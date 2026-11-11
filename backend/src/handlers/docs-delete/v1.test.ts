import { describe, expect, it } from 'vitest';
import { DeletedResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createDocsDeleteHandler } from './v1.js';

const ROUTE = 'DELETE /v1/docs/{id}';

describe('docs-delete handler v1', () => {
  it('answers {deleted: true} for the path id', async () => {
    const seen: unknown[] = [];
    const handler = createDocsDeleteHandler({
      http: httpDeps(),
      deleteDoc: (p, id) => (seen.push([p, id]), Promise.resolve(ok(undefined))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }));

    expect([res.statusCode, DeletedResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { deleted: true },
      [[{ uid: 'kristoff' }, 'aaaaaaaaaaaaaaaa']],
    ]);
  });

  it('answers 404 DOC_NOT_FOUND', async () => {
    const handler = createDocsDeleteHandler({
      http: httpDeps(),
      deleteDoc: () => Promise.resolve(err(appError('DOC_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'aaaaaaaaaaaaaaaa' } }))).statusCode,
    ).toBe(404);
  });
});
