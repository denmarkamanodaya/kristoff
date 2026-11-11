// POST /v1/docs handler (docs/contract.md "Routes"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { DocUpload, DocUploadInput } from '../../usecases/create-doc-upload/index.js';
import { CreateDocRequest } from '../../contracts/requests.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { docsCreateDeps } from '../../composition/docs-create.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, bodyRoute, toBody } from '../../shared/http/routes.js';

/** What the docs-create handler needs. */
export interface DocsCreateHandlerDeps {
  readonly http: HttpDeps;
  readonly createDocUpload: UseCase<[Principal, DocUploadInput], DocUpload>;
}

/**
 * Builds the docs-create handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{doc, uploadUrl, uploadHeaders}`, 400 or 401.
 */
export const createDocsCreateHandler = (deps: DocsCreateHandlerDeps) =>
  bodyRoute(deps.http, 'docs-create', CreateDocRequest, async (principal, body) =>
    toBody(await deps.createDocUpload(principal, body)),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createDocsCreateHandler(await docsCreateDeps()));
