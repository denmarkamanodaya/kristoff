// POST /v1/docs/{id}/confirm handler (docs/contract.md "Routes"): thin driving adapter.
import type { Doc, Principal } from '../../domain/model.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsConfirmDeps } from '../../composition/docs-confirm.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, pathRoute, toBody } from '../../shared/http/routes.js';

/** What the docs-confirm handler needs. */
export interface DocsConfirmHandlerDeps {
  readonly http: HttpDeps;
  readonly confirmDoc: UseCase<[Principal, string], Doc>;
}

/**
 * Builds the docs-confirm handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{doc}` (ready), 400, 401, 404 `DOC_NOT_FOUND`, 422 `UPLOAD_INVALID`.
 */
export const createDocsConfirmHandler = (deps: DocsConfirmHandlerDeps) =>
  pathRoute(deps.http, 'docs-confirm', async (principal, id) =>
    toBody(await deps.confirmDoc(principal, id), (doc) => ({ doc })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsConfirmHandler(await docsConfirmDeps()));
