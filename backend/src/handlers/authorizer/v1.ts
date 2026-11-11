// HTTP API Lambda REQUEST authorizer (docs/contract.md "Auth flow" step 3, "Build artefacts"):
// simple responses; verifies the session and passes `sub` in the context. Never throws: any
// failure is a deny.
import type {
  APIGatewayRequestAuthorizerEventV2,
  APIGatewaySimpleAuthorizerWithContextResult,
} from 'aws-lambda';
import type { Principal } from '../../domain/model.js';
import { authorizerDeps } from '../../composition/authorizer.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import type { Logger } from '../../shared/log/logger.js';
import type { Result } from '../../shared/result/result.js';

/** The context handed to every route (read by src/shared/http). */
interface Context {
  readonly sub: string;
}

/** What the authorizer needs. */
export interface AuthorizerDeps {
  readonly log: Logger;
  readonly verifySession: (
    authorization: string | undefined,
  ) => Promise<Result<Principal, AppError>>;
}

const headerOf = (event: APIGatewayRequestAuthorizerEventV2): string | undefined => {
  const value: unknown = (event as { headers?: Record<string, unknown> | null }).headers
    ?.authorization;
  return typeof value === 'string' ? value : undefined;
};

/**
 * Builds the authorizer.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Logger and the verify-session use case.
 * @returns The handler: `{isAuthorized: true, context: {sub}}` or `{isAuthorized: false}`.
 */
export const createAuthorizer =
  (deps: AuthorizerDeps) =>
  async (
    event: APIGatewayRequestAuthorizerEventV2,
  ): Promise<APIGatewaySimpleAuthorizerWithContextResult<Context>> => {
    const principal = await deps.verifySession(headerOf(event));
    if (principal.ok) return { isAuthorized: true, context: { sub: principal.value.uid } };
    deps.log.warn('auth.denied', { operation: 'authorizer', code: principal.error.code });
    return { isAuthorized: false, context: { sub: '' } };
  };

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createAuthorizer(await authorizerDeps()));
