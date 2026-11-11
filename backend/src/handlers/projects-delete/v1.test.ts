import { describe, expect, it } from 'vitest';
import { DeletedResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createProjectsDeleteHandler } from './v1.js';

const ROUTE = 'DELETE /v1/projects/{id}';

describe('projects-delete handler v1', () => {
  it('answers {deleted: true} for the path id', async () => {
    const seen: unknown[] = [];
    const handler = createProjectsDeleteHandler({
      http: httpDeps(),
      deleteProject: (p, id) => (seen.push([p, id]), Promise.resolve(ok(undefined))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'abababababababab' } }));

    expect([res.statusCode, DeletedResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { deleted: true },
      [[{ uid: 'kristoff' }, 'abababababababab']],
    ]);
  });

  it('answers 404 PROJECT_NOT_FOUND', async () => {
    const handler = createProjectsDeleteHandler({
      http: httpDeps(),
      deleteProject: () => Promise.resolve(err(appError('PROJECT_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'abababababababab' } }))).statusCode,
    ).toBe(404);
  });
});
