// Bounded concurrency for independent async work (copied from dexfs; used by the chat delete): at most `limit` calls
// in flight, results in input order. Pure helper with no business meaning (core/dry.md).

/**
 * Maps items through an async function with at most `limit` calls running at once.
 * @param items - Inputs.
 * @param limit - Most calls in flight, at least 1.
 * @param fn - The work for one item (and its position).
 * @returns The results in input order; rejects with the first rejection.
 * @throws RangeError when `limit` is not a positive integer.
 */
export const mapLimit = async <T, R>(
  items: readonly T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>,
): Promise<R[]> => {
  if (!Number.isInteger(limit) || limit < 1) throw new RangeError('limit must be >= 1');
  const results = new Array<R>(items.length);
  let next = 0;
  const worker = async (): Promise<void> => {
    while (next < items.length) {
      const at = next;
      next += 1;
      results[at] = await fn(items[at] as T, at);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
};
