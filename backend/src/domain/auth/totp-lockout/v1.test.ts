import { describe, expect, it } from 'vitest';
import { INITIAL_TOTP_STATE } from '../totp-state.js';
import { afterFailure, lockSecondsLeft } from './v1.js';

const NOW = 1_800_000_000_000;

describe('totp-lockout v1', () => {
  it('counts the first four failures without locking', () => {
    const four = [1, 2, 3, 4].reduce((s) => afterFailure(s, NOW), INITIAL_TOTP_STATE);

    expect(four).toEqual({ lastStep: 0, failCount: 4, lockedUntil: 0 });
  });

  it('locks for 15 minutes on the fifth consecutive failure and resets the count', () => {
    const state = { lastStep: 7, failCount: 4, lockedUntil: 0 };

    expect(afterFailure(state, NOW)).toEqual({
      lastStep: 7,
      failCount: 0,
      lockedUntil: NOW + 900_000,
    });
  });

  it('reports the seconds left on a lock, rounded up, and 0 once it has passed', () => {
    const locked = { lastStep: 0, failCount: 0, lockedUntil: NOW + 1500 };

    expect([lockSecondsLeft(locked, NOW), lockSecondsLeft(locked, NOW + 1500)]).toEqual([2, 0]);
  });
});
