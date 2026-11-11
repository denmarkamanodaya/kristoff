// DELETE /v1/projects/{id} handler (docs/contract.md "v2 Projects"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import { type HttpDeps } from '../../shared/http/http.js';
import { projectsDeleteDeps } from '../../composition/projects-delete.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { pathRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the projects-delete handler needs. */
export interface ProjectsDeleteHandlerDeps {
  readonly http: HttpDeps;
  readonly deleteProject: UseCase<[Principal, string], void>;
}

/**
 * Builds the projects-delete handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{deleted: true}`, 400, 401 or 404 `PROJECT_NOT_FOUND`.
 */
export const createProjectsDeleteHandler = (deps: ProjectsDeleteHandlerDeps) =>
  pathRoute(deps.http, 'projects-delete', async (principal, id) =>
    toBody(await deps.deleteProject(principal, id), () => ({ deleted: true })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () =>
  createProjectsDeleteHandler(await projectsDeleteDeps()),
);
