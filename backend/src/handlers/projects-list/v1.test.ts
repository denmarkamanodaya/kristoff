import { describe, expect, it } from 'vitest';
import { ProjectsResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createProjectsListHandler } from './v1.js';

const P = {
  id: 'abababababababab',
  name: 'Torts',
  createdAt: '2026-11-12T00:00:00.000Z',
  updatedAt: '2026-11-12T00:00:00.000Z',
};

describe('projects-list handler v1', () => {
  it('answers {items} for the principal', async () => {
    const seen: unknown[] = [];
    const handler = createProjectsListHandler({
      http: httpDeps(),
      listProjects: (p) => (seen.push(p), Promise.resolve(ok([P]))),
    });

    const res = await handler(httpEvent({ routeKey: 'GET /v1/projects' }));

    expect([res.statusCode, ProjectsResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { items: [P] },
      [{ uid: 'kristoff' }],
    ]);
  });
});
