// The one backend error class (stack/errors-and-resilience.md). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md (copied from dexfs).
// It carries a stable code from the contract list and never an HTTP status: the core does not know
// the transport. Handlers map the code to the envelope and status (contracts/errors.ts).
import type { ErrorCode } from '../../contracts/errors.js';

/** Safe, bounded extra values attached to an error; never secrets, keys or another user's id. */
export type ErrorDetails = Readonly<Record<string, string | number>>;

/** A typed error with a stable `UPPER_SNAKE` code callers switch on; the message is for humans. */
export class AppError extends Error {
  /** Stable identifier from the contract's error-code list. */
  readonly code: ErrorCode;
  /** Optional safe, bounded context for logs. */
  readonly details: ErrorDetails | undefined;

  /**
   * Builds a typed error.
   * @param code - Contract error code.
   * @param message - Human-readable description; may change between releases.
   * @param details - Optional safe, bounded context.
   */
  constructor(code: ErrorCode, message: string, details?: ErrorDetails) {
    super(message);
    this.name = 'AppError';
    this.code = code;
    this.details = details;
  }
}

/**
 * Builds an {@link AppError} without `new`, for use inside `err(...)`.
 * @param code - Contract error code.
 * @param message - Human-readable description.
 * @param details - Optional safe, bounded context.
 * @returns The error.
 */
export const appError = (code: ErrorCode, message: string, details?: ErrorDetails): AppError =>
  new AppError(code, message, details);
