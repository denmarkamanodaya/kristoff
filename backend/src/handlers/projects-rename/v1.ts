// PATCH /v1/projects/{id} handler (docs/contract.md "v2 Projects"): thin driving adapter.
import type { Principal, Project } from '../../domain/model.js';
import { ProjectNameRequest } from '../../contracts/requests.js';
import { type HttpDeps } from '../../shared/http/http.js';
import { projectsRenameDeps } from '../../composition/projects-rename.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { pathBodyRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the projects-rename handler needs. */
export interface ProjectsRenameHandlerDeps {
  readonly http: HttpDeps;
  readonly renameProject: UseCase<[Principal, string, string], Project>;
}

/**
 * Builds the projects-rename handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{project}`, 400, 401 or 404 `PROJECT_NOT_FOUND`.
 */
export const createProjectsRenameHandler = (deps: ProjectsRenameHandlerDeps) =>
  pathBodyRoute(deps.http, 'projects-rename', ProjectNameRequest, async (principal, id, body) =>
    toBody(await deps.renameProject(principal, id, body.name), (project) => ({ project })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () =>
  createProjectsRenameHandler(await projectsRenameDeps()),
);
