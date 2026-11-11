// SystemClock: the Clock port on the host clock (the only place `Date.now()` is called).
import type { Clock } from '../../ports/clock.js';

/**
 * The host clock.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 */
export const systemClock: Clock = { now: () => Date.now() };
