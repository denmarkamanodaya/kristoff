// DocSplitTrigger port: starts docs-split for one document asynchronously (docs/contract.md
// "v2 Large PDFs"). Delivery is at least once; docs-split skips a doc that is no longer processing.
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';

/** Starts background processing of an uploaded PDF. */
export interface DocSplitTrigger {
  /** Queues the split; returns once it is accepted (not when it finishes). */
  start(uid: string, docId: string): Promise<Result<void, AppError>>;
}
