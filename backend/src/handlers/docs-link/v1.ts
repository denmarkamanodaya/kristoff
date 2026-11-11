// GET /v1/docs/{id}/link handler (docs/contract.md "Routes"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { PresignedUrl } from '../../ports/file-store.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsLinkDeps } from '../../composition/docs-link.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, pathRoute, toBody } from '../../shared/http/routes.js';

/** What the docs-link handler needs. */
export interface DocsLinkHandlerDeps {
  readonly http: HttpDeps;
  readonly getDocLink: UseCase<[Principal, string], PresignedUrl>;
}

/**
 * Builds the docs-link handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{url, expiresAt}`, 400, 401 or 404 `DOC_NOT_FOUND`.
 */
export const createDocsLinkHandler = (deps: DocsLinkHandlerDeps) =>
  pathRoute(deps.http, 'docs-link', async (principal, id) =>
    toBody(await deps.getDocLink(principal, id)),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsLinkHandler(await docsLinkDeps()));
