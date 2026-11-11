// GET /v1/docs/{id}/link handler (docs/contract.md "Routes", "v2 Large PDFs"): thin driving adapter;
// `?part=n` links one part of a large doc.
import type { Principal } from '../../domain/model.js';
import type { PresignedUrl } from '../../ports/file-store.js';
import { LinkQuery } from '../../contracts/requests.js';
import { parseValue, type HttpDeps } from '../../shared/http/http.js';
import { docsLinkDeps } from '../../composition/docs-link.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { pathRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the docs-link handler needs. */
export interface DocsLinkHandlerDeps {
  readonly http: HttpDeps;
  readonly getDocLink: UseCase<[Principal, string, number | undefined], PresignedUrl>;
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
export const createDocsLinkHandler =
  (deps: DocsLinkHandlerDeps): ReturnType<typeof pathRoute> =>
  (event) =>
    pathRoute(deps.http, 'docs-link', async (principal, id) => {
      const query = parseValue(event.queryStringParameters, LinkQuery, 'query');
      if (!query.ok) return query;
      const part = query.value.part === undefined ? undefined : Number(query.value.part);
      return toBody(await deps.getDocLink(principal, id, part));
    })(event);

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsLinkHandler(await docsLinkDeps()));
