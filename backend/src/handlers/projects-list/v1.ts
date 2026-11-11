// GET /v1/projects handler (docs/contract.md "v2 Projects"): thin driving adapter.
import type { Principal, Project } from '../../domain/model.js';
import {
  runRoute,
  type HttpDeps,
  type HttpEvent,
  type HttpResult,
} from '../../shared/http/http.js';
import { projectsListDeps } from '../../composition/projects-list.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { toBody, type UseCase } from '../../shared/http/routes.js';

/** What the projects-list handler needs. */
export interface ProjectsListHandlerDeps {
  readonly http: HttpDeps;
  readonly listProjects: UseCase<[Principal], Project[]>;
}

/**
 * Builds the projects-list handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{items}` in name order, or 401.
 */
export const createProjectsListHandler =
  (deps: ProjectsListHandlerDeps) =>
  (event: HttpEvent): Promise<HttpResult> =>
    runRoute(deps.http, 'projects-list', event, async (principal) =>
      toBody(await deps.listProjects(principal), (items) => ({ items })),
    );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createProjectsListHandler(await projectsListDeps()));
