// FlagReader port: the AI kill switch (stack/feature-flags.md). Contract: docs/contract.md
// "Config" (`/kristoff/prod/ai-enabled`, 60 s cache).
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';

/** Reads runtime flags. */
export interface FlagReader {
  /** Whether model calls are allowed; an error when the flag cannot be read or is not true/false. */
  isAiEnabled(): Promise<Result<boolean, AppError>>;
}
