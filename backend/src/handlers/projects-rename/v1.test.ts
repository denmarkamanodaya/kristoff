import { describe, expect, it } from 'vitest';
import { ProjectResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createProjectsRenameHandler } from './v1.js';

const P = {
  id: 'abababababababab',
  name: 'New',
  createdAt: '2026-11-12T00:00:00.000Z',
  updatedAt: '2026-11-12T00:00:00.000Z',
};
const ROUTE = 'PATCH /v1/projects/{id}';
const path = { id: 'abababababababab' };

describe('projects-rename handler v1', () => {
  it('passes id and name and answers {project}', async () => {
    const seen: unknown[] = [];
    const handler = createProjectsRenameHandler({
      http: httpDeps(),
      renameProject: (p, id, n) => (seen.push([p, id, n]), Promise.resolve(ok(P))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path, body: { name: 'New' } }));

    expect([res.statusCode, ProjectResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { project: P },
      [[{ uid: 'kristoff' }, P.id, 'New']],
    ]);
  });

  it('answers 404 PROJECT_NOT_FOUND', async () => {
    const handler = createProjectsRenameHandler({
      http: httpDeps(),
      renameProject: () => Promise.resolve(err(appError('PROJECT_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path, body: { name: 'x' } }))).statusCode,
    ).toBe(404);
  });
});
