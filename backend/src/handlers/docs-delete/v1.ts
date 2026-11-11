// DELETE /v1/docs/{id} handler (docs/contract.md "v2 File delete"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsDeleteDeps } from '../../composition/docs-delete.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { pathRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the docs-delete handler needs. */
export interface DocsDeleteHandlerDeps {
  readonly http: HttpDeps;
  readonly deleteDoc: UseCase<[Principal, string], void>;
}

/**
 * Builds the docs-delete handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{deleted: true}`, 400, 401 or 404 `DOC_NOT_FOUND`.
 */
export const createDocsDeleteHandler = (deps: DocsDeleteHandlerDeps) =>
  pathRoute(deps.http, 'docs-delete', async (principal, id) =>
    toBody(await deps.deleteDoc(principal, id), () => ({ deleted: true })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsDeleteHandler(await docsDeleteDeps()));
