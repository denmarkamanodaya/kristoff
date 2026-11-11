// GET /v1/usage handler (docs/contract.md "Routes"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { UsageReport } from '../../ports/usage-meter.js';
import {
  runRoute,
  type HttpDeps,
  type HttpEvent,
  type HttpResult,
} from '../../shared/http/http.js';
import { usageGetDeps } from '../../composition/usage-get.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, toBody } from '../../shared/http/routes.js';

/** What the usage-get handler needs. */
export interface UsageGetHandlerDeps {
  readonly http: HttpDeps;
  readonly getUsage: UseCase<[Principal], UsageReport>;
}

/**
 * Builds the usage-get handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{today, total}` or 401.
 */
export const createUsageGetHandler =
  (deps: UsageGetHandlerDeps) =>
  (event: HttpEvent): Promise<HttpResult> =>
    runRoute(deps.http, 'usage-get', event, async (principal) =>
      toBody(await deps.getUsage(principal)),
    );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createUsageGetHandler(await usageGetDeps()));
