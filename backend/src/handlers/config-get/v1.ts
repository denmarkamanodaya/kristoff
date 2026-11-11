// GET /v1/config handler (docs/contract.md "Routes"): thin driving adapter.
import type { ClientConfig } from '../../usecases/get-config/index.js';
import {
  runRoute,
  type HttpDeps,
  type HttpEvent,
  type HttpResult,
} from '../../shared/http/http.js';
import { configGetDeps } from '../../composition/config-get.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { ok } from '../../shared/result/result.js';

/** What the config-get handler needs. */
export interface ConfigGetHandlerDeps {
  readonly http: HttpDeps;
  readonly getConfig: () => ClientConfig;
}

/**
 * Builds the config-get handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{model, maxAttachedDocs, dailyQuestionCap}` or 401.
 */
export const createConfigGetHandler =
  (deps: ConfigGetHandlerDeps) =>
  (event: HttpEvent): Promise<HttpResult> =>
    runRoute(deps.http, 'config-get', event, () => Promise.resolve(ok({ body: deps.getConfig() })));

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createConfigGetHandler(await configGetDeps()));
