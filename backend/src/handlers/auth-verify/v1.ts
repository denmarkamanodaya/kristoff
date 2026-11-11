// POST /v1/auth/verify handler (docs/contract.md "Routes"): public route; thin driving adapter.
import type { Session } from '../../usecases/verify-otp/index.js';
import { VerifyRequest } from '../../contracts/requests.js';
import { authVerifyDeps } from '../../composition/auth-verify.js';
import type { AppError } from '../../shared/errors/app-error.js';
import {
  parseJson,
  runPublicRoute,
  type HttpDeps,
  type HttpEvent,
  type HttpResult,
} from '../../shared/http/http.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { ok, type Result } from '../../shared/result/result.js';

/** What the auth-verify handler needs. */
export interface AuthVerifyHandlerDeps {
  readonly http: HttpDeps;
  readonly verifyOtp: (code: string) => Promise<Result<Session, AppError>>;
}

/**
 * Builds the auth-verify handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{token, expiresAt}`, 400, 401 `OTP_INVALID`/`OTP_REPLAYED`, 429 `OTP_LOCKED`.
 */
export const createAuthVerifyHandler =
  (deps: AuthVerifyHandlerDeps) =>
  (event: HttpEvent): Promise<HttpResult> =>
    runPublicRoute(deps.http, 'auth-verify', event, async () => {
      const body = parseJson(event.body, event.isBase64Encoded, VerifyRequest);
      if (!body.ok) return body;
      const session = await deps.verifyOtp(body.value.code);
      return session.ok ? ok({ body: session.value }) : session;
    });

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createAuthVerifyHandler(await authVerifyDeps()));
