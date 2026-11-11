// Adds two token usages field by field (the answer call plus the follow-up call are counted
// together; docs/contract.md "v2 Answer extras"). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Usage } from '../../model.js';

/**
 * Sums two usages.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param a - A usage.
 * @param b - Another usage, or null (counts as zero).
 * @returns The field-wise sum.
 */
export const sumUsage = (a: Usage, b: Usage | null): Usage =>
  b === null
    ? a
    : {
        input: a.input + b.input,
        output: a.output + b.output,
        cacheRead: a.cacheRead + b.cacheRead,
        cacheWrite: a.cacheWrite + b.cacheWrite,
      };
