// Deterministic Clock and IdGenerator fakes.
import type { Clock } from '../../../src/ports/clock.js';
import type { IdGenerator } from '../../../src/ports/id-generator.js';

/** A clock that stays at `start` until `advance` moves it. */
export const fakeClock = (start = Date.UTC(2026, 10, 12, 1, 0, 0)) => {
  let now = start;
  return {
    now: () => now,
    advance: (ms: number) => {
      now += ms;
    },
  } satisfies Clock & { advance: (ms: number) => void };
};

/** Sequential ids: `0000000000000001`, ... and ULID-shaped `01J...0001`, ... */
export const fakeIds = (): IdGenerator => {
  let n = 0;
  let u = 0;
  return {
    newId: () => (n += 1).toString(16).padStart(16, '0'),
    newUlid: () => `01J${(u += 1).toString().padStart(23, '0')}`,
  };
};
