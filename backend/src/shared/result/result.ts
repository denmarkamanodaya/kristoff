// The shared Result type (stack/errors-and-resilience.md, ruling C5). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md (copied from dexfs).
// Expected failures are values; only programmer errors throw.

/** Success variant of {@link Result}. */
export interface Ok<T> {
  readonly ok: true;
  readonly value: T;
}

/** Failure variant of {@link Result}. */
export interface Err<E> {
  readonly ok: false;
  readonly error: E;
}

/** Outcome of an operation that can fail in an expected way: exactly one of `Ok` or `Err`. */
export type Result<T, E> = Ok<T> | Err<E>;

/**
 * Wraps a success value.
 * @param value - The value carried by the `Ok` variant.
 * @returns An `Ok` result.
 */
export const ok = <T>(value: T): Ok<T> => ({ ok: true, value });

/**
 * Wraps an expected failure.
 * @param error - The typed error carried by the `Err` variant.
 * @returns An `Err` result.
 */
export const err = <E>(error: E): Err<E> => ({ ok: false, error });
