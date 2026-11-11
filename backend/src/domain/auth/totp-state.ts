// The owner's stored TOTP state (docs/contract.md "Auth flow" step 5). A plain value type.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** Replay and lockout state. `lastStep` 0 = no code used yet; `lockedUntil` 0 = not locked. */
export interface TotpState {
  /** Last accepted 30 s time step (epoch seconds / 30). */
  readonly lastStep: number;
  /** Consecutive failed codes since the last success or lock. */
  readonly failCount: number;
  /** Epoch milliseconds until which every code is refused. */
  readonly lockedUntil: number;
}

/** The state of an owner who never entered a code. */
export const INITIAL_TOTP_STATE: TotpState = { lastStep: 0, failCount: 0, lockedUntil: 0 };
