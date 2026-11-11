// POST /v1/auth/verify use case (docs/contract.md "Auth flow"): lock check, RFC 6238 match with
// replay guard, lockout after 5 failures, then a signed session token. Fails closed on any
// lookup error. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { afterFailure, lockSecondsLeft } from '../../domain/auth/totp-lockout/index.js';
import { signSession } from '../../domain/auth/session-token/index.js';
import { decodeBase32, matchTotpStep } from '../../domain/auth/verify-totp/index.js';
import type { TotpState } from '../../domain/auth/totp-state.js';
import type { Versioned } from '../../domain/model.js';
import type { AuthStateStore } from '../../ports/auth-state-store.js';
import type { Clock } from '../../ports/clock.js';
import type { SecretSource } from '../../ports/secret-source.js';
import { RETRY_AFTER_DETAIL } from '../../contracts/errors.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** What verify-otp needs. */
export interface VerifyOtpDeps {
  readonly authState: AuthStateStore;
  readonly secrets: SecretSource;
  readonly clock: Clock;
  /** The single owner id the token's `sub` carries (config `OWNER_ID`). */
  readonly ownerId: string;
  /** Session lifetime in seconds (config `SESSION_TTL_SECONDS`). */
  readonly sessionTtlSeconds: number;
}

/** A new session. `expiresAt` is an ISO instant. */
export interface Session {
  readonly token: string;
  readonly expiresAt: string;
}

const locked = (seconds: number): AppError =>
  appError('OTP_LOCKED', 'too many wrong codes; wait', { [RETRY_AFTER_DETAIL]: seconds });

const fail = async (
  deps: VerifyOtpDeps,
  stored: Versioned<TotpState>,
  now: number,
): Promise<Result<never, AppError>> => {
  const next = afterFailure(stored.value, now);
  const saved = await deps.authState.recordFailure(deps.ownerId, next, stored.version);
  if (!saved.ok) return saved;
  const seconds = lockSecondsLeft(next, now);
  return err(seconds > 0 ? locked(seconds) : appError('OTP_INVALID', 'wrong code'));
};

const issue = async (deps: VerifyOtpDeps, now: number): Promise<Result<Session, AppError>> => {
  const key = await deps.secrets.get('session-key');
  if (!key.ok) return key;
  const iat = Math.floor(now / 1000);
  const exp = iat + deps.sessionTtlSeconds;
  const token = signSession(Buffer.from(key.value, 'base64'), { sub: deps.ownerId, iat, exp });
  return ok({ token, expiresAt: new Date(exp * 1000).toISOString() });
};

/**
 * Builds the verify-otp use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - TOTP state store, secrets, clock, owner id and session lifetime.
 * @returns A function from the six-digit code to a session, or `OTP_LOCKED` (with the seconds left),
 *   `OTP_INVALID`, `OTP_REPLAYED`, `VERSION_CONFLICT`, `INTERNAL` or `UPSTREAM_UNAVAILABLE`.
 */
export const verifyOtp =
  (deps: VerifyOtpDeps) =>
  async (code: string): Promise<Result<Session, AppError>> => {
    const now = deps.clock.now();
    const stored = await deps.authState.get(deps.ownerId);
    if (!stored.ok) return stored;
    const seconds = lockSecondsLeft(stored.value.value, now);
    if (seconds > 0) return err(locked(seconds));
    const secretText = await deps.secrets.get('totp-secret');
    if (!secretText.ok) return secretText;
    const secret = decodeBase32(secretText.value);
    if (secret === undefined) return err(appError('INTERNAL', 'TOTP secret is not base32'));
    const step = matchTotpStep(secret, code, now);
    if (step === undefined) return fail(deps, stored.value, now);
    const accepted = await deps.authState.acceptStep(deps.ownerId, step);
    return accepted.ok ? issue(deps, now) : accepted;
  };
