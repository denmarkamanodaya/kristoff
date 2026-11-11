// Builds a Lambda entry that wires its dependencies once per container, on the first invocation
// (pure helper, no business meaning). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md (copied from dexfs). A failed wiring (bad env, unreadable
// secret) is rethrown and not cached, so the next invocation tries again instead of serving a
// broken container forever.

/** What a handler may read from the Lambda context: the time this invocation has left. */
export interface InvocationContext {
  getRemainingTimeInMillis(): number;
}

/**
 * Wraps an async factory into a Lambda entry. The Lambda context is passed through, so a handler
 * can bound its work by the remaining time.
 * @param build - Creates the real handler (reads env, builds clients); called at most once per
 *   container while it succeeds.
 * @returns The entry function Lambda calls.
 */
export const lazyHandler = <E, R>(
  build: () => Promise<(event: E, context?: InvocationContext) => Promise<R>>,
): ((event: E, context?: InvocationContext) => Promise<R>) => {
  let live: Promise<(event: E, context?: InvocationContext) => Promise<R>> | undefined;
  return async (event: E, context?: InvocationContext): Promise<R> => {
    live ??= build().catch((error: unknown) => {
      live = undefined;
      throw error;
    });
    const run = await live;
    return run(event, context);
  };
};
