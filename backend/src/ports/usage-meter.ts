// UsageMeter port: the daily question cap and token counters.
// Contract: docs/contract.md "Storage" (Usage day/total) and the daily-cap rule.
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { Usage } from '../domain/model.js';

/** Today's and all-time token counters. */
export interface UsageReport {
  readonly today: Usage;
  readonly total: Usage;
}

/** Meters model calls per owner and UTC day. */
export interface UsageMeter {
  /**
   * Counts one question for `day` in a single conditional write, before the model call.
   * `DAILY_CAP_REACHED` when `cap` questions were already counted.
   */
  admitQuestion(uid: string, day: string, cap: number): Promise<Result<void, AppError>>;
  /** Adds a call's tokens to the day and the total counters. */
  addTokens(uid: string, day: string, usage: Usage): Promise<Result<void, AppError>>;
  /** Reads the counters (zeros when nothing was counted). */
  read(uid: string, day: string): Promise<Result<UsageReport, AppError>>;
}
