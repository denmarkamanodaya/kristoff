import { describe, expect, it } from 'vitest';
import { ProjectResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createProjectsCreateHandler } from './v1.js';

const P = {
  id: 'abababababababab',
  name: 'Torts',
  createdAt: '2026-11-12T00:00:00.000Z',
  updatedAt: '2026-11-12T00:00:00.000Z',
};
const ROUTE = 'POST /v1/projects';

describe('projects-create handler v1', () => {
  it('passes the name and answers {project}', async () => {
    const seen: unknown[] = [];
    const handler = createProjectsCreateHandler({
      http: httpDeps(),
      createProject: (p, n) => (seen.push([p, n]), Promise.resolve(ok(P))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, body: { name: 'Torts' } }));

    expect([res.statusCode, ProjectResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      { project: P },
      [[{ uid: 'kristoff' }, 'Torts']],
    ]);
  });

  it('answers 409 PROJECT_LIMIT_REACHED and 400 for an 81-char name', async () => {
    const handler = createProjectsCreateHandler({
      http: httpDeps(),
      createProject: () => Promise.resolve(err(appError('PROJECT_LIMIT_REACHED', 'cap'))),
    });

    expect((await handler(httpEvent({ routeKey: ROUTE, body: { name: 'x' } }))).statusCode).toBe(
      409,
    );
    expect(
      (await handler(httpEvent({ routeKey: ROUTE, body: { name: 'x'.repeat(81) } }))).statusCode,
    ).toBe(400);
  });
});
