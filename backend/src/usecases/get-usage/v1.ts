// GET /v1/usage use case (docs/contract.md "Routes"): today's (UTC) and all-time token counters.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { usageDay } from '../../domain/usage/usage-day/index.js';
import type { Principal } from '../../domain/model.js';
import type { Clock } from '../../ports/clock.js';
import type { UsageMeter, UsageReport } from '../../ports/usage-meter.js';
import type { AppError } from '../../shared/errors/app-error.js';
import type { Result } from '../../shared/result/result.js';

/**
 * Builds the get-usage use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Usage meter and clock.
 * @returns A function from the principal to the counters; meter failures pass through.
 */
export const getUsage =
  (deps: { readonly meter: UsageMeter; readonly clock: Clock }) =>
  (principal: Principal): Promise<Result<UsageReport, AppError>> =>
    deps.meter.read(principal.uid, usageDay(deps.clock.now()));
