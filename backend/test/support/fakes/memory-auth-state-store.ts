// In-memory AuthStateStore; trusted because test/ports runs the same contract suite.
import { INITIAL_TOTP_STATE, type TotpState } from '../../../src/domain/auth/totp-state.js';
import type { AuthStateStore } from '../../../src/ports/auth-state-store.js';
import { appError } from '../../../src/shared/errors/app-error.js';
import { err, ok } from '../../../src/shared/result/result.js';

/** The fake plus its raw rows. */
export const memoryAuthStateStore = () => {
  const rows = new Map<string, { state: TotpState; version: number }>();
  const store: AuthStateStore = {
    get: (uid) => {
      const row = rows.get(uid);
      return Promise.resolve(
        ok(
          row
            ? { value: row.state, version: row.version }
            : { value: INITIAL_TOTP_STATE, version: 0 },
        ),
      );
    },
    acceptStep: (uid, step) => {
      const row = rows.get(uid) ?? { state: INITIAL_TOTP_STATE, version: 0 };
      if (row.state.lastStep >= step) return Promise.resolve(err(appError('OTP_REPLAYED', 'used')));
      rows.set(uid, {
        state: { lastStep: step, failCount: 0, lockedUntil: 0 },
        version: row.version + 1,
      });
      return Promise.resolve(ok(undefined));
    },
    recordFailure: (uid, next, version) => {
      const current = rows.get(uid)?.version ?? 0;
      if (current !== version) return Promise.resolve(err(appError('VERSION_CONFLICT', 'stale')));
      rows.set(uid, { state: next, version: version + 1 });
      return Promise.resolve(ok(undefined));
    },
  };
  return { store, rows };
};
