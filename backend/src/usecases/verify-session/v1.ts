// Verifies a session token for the HTTP authorizer and the stream Lambda (docs/contract.md
// "Auth flow" step 3). Fails closed: any problem is UNAUTHENTICATED, and the subject must be the
// configured owner. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { verifySessionToken } from '../../domain/auth/session-token/index.js';
import type { Principal } from '../../domain/model.js';
import type { Clock } from '../../ports/clock.js';
import type { SecretSource } from '../../ports/secret-source.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** What verify-session needs. */
export interface VerifySessionDeps {
  readonly secrets: SecretSource;
  readonly clock: Clock;
  /** The single owner id (config `OWNER_ID`). */
  readonly ownerId: string;
}

const BEARER = /^Bearer ([A-Za-z0-9_-]{1,1024}\.[A-Za-z0-9_-]{1,128})$/;

/**
 * Builds the verify-session use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Secrets, clock and owner id.
 * @returns A function from the raw `Authorization` header to the principal, or `UNAUTHENTICATED`
 *   (missing, malformed, forged, expired or foreign token; also when the key cannot be read).
 */
export const verifySession =
  (deps: VerifySessionDeps) =>
  async (authorization: string | undefined): Promise<Result<Principal, AppError>> => {
    const denied = err(appError('UNAUTHENTICATED', 'sign in again'));
    const token = BEARER.exec(authorization ?? '')?.[1];
    if (token === undefined) return denied;
    const key = await deps.secrets.get('session-key');
    if (!key.ok) return denied;
    const nowSeconds = Math.floor(deps.clock.now() / 1000);
    const payload = verifySessionToken(Buffer.from(key.value, 'base64'), token, nowSeconds);
    return payload?.sub === deps.ownerId ? ok({ uid: payload.sub }) : denied;
  };
