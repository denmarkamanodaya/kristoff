// TOTP lockout rule (docs/contract.md "Auth flow" step 5): 5 consecutive failures lock every code
// for 15 minutes; a success resets the count. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { TotpState } from '../totp-state.js';

/** Consecutive failures that trigger a lock. */
const MAX_FAILURES = 5;
/** Lock length in milliseconds (15 minutes). */
const LOCK_MS = 15 * 60 * 1000;

/**
 * Seconds left on a lock.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param state - Stored TOTP state.
 * @param nowMs - Epoch milliseconds from the injected clock.
 * @returns Whole seconds until the lock ends (rounded up), or 0 when not locked.
 */
export const lockSecondsLeft = (state: TotpState, nowMs: number): number =>
  state.lockedUntil > nowMs ? Math.ceil((state.lockedUntil - nowMs) / 1000) : 0;

/**
 * The state after one wrong code. The fifth consecutive failure locks and resets the count.
 * @param state - Stored TOTP state.
 * @param nowMs - Epoch milliseconds from the injected clock.
 * @returns The next state to write.
 */
export const afterFailure = (state: TotpState, nowMs: number): TotpState => {
  const failCount = state.failCount + 1;
  return failCount >= MAX_FAILURES
    ? { ...state, failCount: 0, lockedUntil: nowMs + LOCK_MS }
    : { ...state, failCount };
};
