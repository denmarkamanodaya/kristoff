// AuthStateStore port: the owner's TOTP replay and lockout state.
// Contract: docs/contract.md "Auth flow" step 5 and "Storage" (TOTP state).
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { TotpState } from '../domain/auth/totp-state.js';
import type { Versioned } from '../domain/model.js';

/** Stores TOTP state with conditional writes. */
export interface AuthStateStore {
  /** The stored state; version 0 with the initial state when nothing is stored yet. */
  get(uid: string): Promise<Result<Versioned<TotpState>, AppError>>;
  /**
   * Accepts `step`: sets `lastStep`, clears failures and lock, only if the stored `lastStep` is
   * lower. `OTP_REPLAYED` when it is not (the code or a later one was already used).
   */
  acceptStep(uid: string, step: number): Promise<Result<void, AppError>>;
  /** Writes the state after a failure if the version is unchanged; else `VERSION_CONFLICT`. */
  recordFailure(uid: string, next: TotpState, version: number): Promise<Result<void, AppError>>;
}
