// The usage-day rule (docs/contract.md "Storage": `USAGE#<yyyy-mm-dd UTC>`): the daily cap resets at
// 00:00 UTC. Time always comes from the injected Clock. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

const DAY_MS = 86_400_000;

/**
 * The UTC calendar day that contains an instant.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param nowMs - Epoch milliseconds from the injected clock.
 * @returns The UTC date `yyyy-mm-dd`.
 */
export const usageDay = (nowMs: number): string => new Date(nowMs).toISOString().slice(0, 10);

/**
 * Seconds until the next UTC day starts (the `Retry-After` of `DAILY_CAP_REACHED`).
 * @param nowMs - Epoch milliseconds from the injected clock.
 * @returns Whole seconds, at least 1.
 */
export const secondsUntilNextUsageDay = (nowMs: number): number =>
  Math.max(1, Math.ceil((DAY_MS - (nowMs % DAY_MS)) / 1000));
