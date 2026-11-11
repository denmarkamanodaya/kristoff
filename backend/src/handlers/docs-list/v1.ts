// GET /v1/docs handler (docs/contract.md "Routes"): thin driving adapter.
import type { Doc, Page, Principal } from '../../domain/model.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsListDeps } from '../../composition/docs-list.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, listRoute, toBody } from '../../shared/http/routes.js';

/** What the docs-list handler needs. */
export interface DocsListHandlerDeps {
  readonly http: HttpDeps;
  readonly listDocs: UseCase<[Principal, string | undefined], Page<Doc>>;
}

/**
 * Builds the docs-list handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{items, nextCursor}`, 400 for a bad cursor, or 401.
 */
export const createDocsListHandler = (deps: DocsListHandlerDeps) =>
  listRoute(deps.http, 'docs-list', async (principal, cursor) =>
    toBody(await deps.listDocs(principal, cursor)),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsListHandler(await docsListDeps()));
