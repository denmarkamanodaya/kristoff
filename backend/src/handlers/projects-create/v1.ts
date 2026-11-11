// POST /v1/projects handler (docs/contract.md "v2 Projects"): thin driving adapter.
import type { Principal, Project } from '../../domain/model.js';
import { ProjectNameRequest } from '../../contracts/requests.js';
import { type HttpDeps } from '../../shared/http/http.js';
import { projectsCreateDeps } from '../../composition/projects-create.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { bodyRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the projects-create handler needs. */
export interface ProjectsCreateHandlerDeps {
  readonly http: HttpDeps;
  readonly createProject: UseCase<[Principal, string], Project>;
}

/**
 * Builds the projects-create handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{project}`, 400, 401 or 409 `PROJECT_LIMIT_REACHED`.
 */
export const createProjectsCreateHandler = (deps: ProjectsCreateHandlerDeps) =>
  bodyRoute(deps.http, 'projects-create', ProjectNameRequest, async (principal, body) =>
    toBody(await deps.createProject(principal, body.name), (project) => ({ project })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () =>
  createProjectsCreateHandler(await projectsCreateDeps()),
);
