// Translating a vendor (SDK, library) exception at an adapter boundary (core/architecture-hexagonal.md
// "Adapters translate"): the exception's type name is kept for logs, nothing else crosses the port.
// Shared kernel, no business meaning. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { appError, type AppError } from './app-error.js';

/**
 * The bounded type name of a thrown value, for the `error.type` log field.
 * @param error - What a vendor call threw or rejected with.
 * @returns The error's `name` (≤ 64 chars), or `Unknown` for a non-Error.
 */
export const errorTypeOf = (error: unknown): string =>
  error instanceof Error ? error.name.slice(0, 64) : 'Unknown';

/**
 * `UPSTREAM_UNAVAILABLE` for a failed vendor call.
 * @param operation - The call, e.g. `s3.GetObject`.
 * @param error - What it threw.
 * @param message - Human text (defaults to `<operation> failed`).
 * @returns The typed error with `operation` and `error.type` details.
 */
export const upstreamError = (
  operation: string,
  error: unknown,
  message = `${operation} failed`,
): AppError =>
  appError('UPSTREAM_UNAVAILABLE', message, { operation, 'error.type': errorTypeOf(error) });
