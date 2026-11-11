// PATCH /v1/docs/{id} handler (docs/contract.md "Routes"): thin driving adapter.
import type { Doc, Principal } from '../../domain/model.js';
import { RenameDocRequest } from '../../contracts/requests.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsRenameDeps } from '../../composition/docs-rename.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, pathBodyRoute, toBody } from '../../shared/http/routes.js';

/** What the docs-rename handler needs. */
export interface DocsRenameHandlerDeps {
  readonly http: HttpDeps;
  readonly renameDoc: UseCase<[Principal, string, string], Doc>;
}

/**
 * Builds the docs-rename handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{doc}`, 400, 401 or 404 `DOC_NOT_FOUND`.
 */
export const createDocsRenameHandler = (deps: DocsRenameHandlerDeps) =>
  pathBodyRoute(deps.http, 'docs-rename', RenameDocRequest, async (principal, id, body) =>
    toBody(await deps.renameDoc(principal, id, body.title), (doc) => ({ doc })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsRenameHandler(await docsRenameDeps()));
