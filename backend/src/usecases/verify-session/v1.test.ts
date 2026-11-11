import { describe, expect, it } from 'vitest';
import { signSession } from '../../domain/auth/session-token/index.js';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { fixedSecrets } from '../../../test/support/fakes/fixed-ports.js';
import { verifySession } from './v1.js';

const KEY = Buffer.alloc(32, 7);
const NOW = Date.UTC(2026, 10, 12, 1, 0, 0);
const NOW_S = NOW / 1000;

const run = verifySession({
  secrets: fixedSecrets({ 'session-key': KEY.toString('base64') }),
  clock: fakeClock(NOW),
  ownerId: 'kristoff',
});

const bearer = (sub: string, exp: number, key = KEY) =>
  `Bearer ${signSession(key, { sub, iat: NOW_S - 10, exp })}`;

describe('verify-session v1', () => {
  it('returns the owner principal for a valid bearer token', async () => {
    expect(await run(bearer('kristoff', NOW_S + 60))).toEqual({
      ok: true,
      value: { uid: 'kristoff' },
    });
  });

  it.each([
    ['missing header', undefined],
    ['no Bearer prefix', 'abc.def'],
    ['expired', bearer('kristoff', NOW_S)],
    ['forged with another key', bearer('kristoff', NOW_S + 60, Buffer.alloc(32, 9))],
    ['another subject', bearer('eve', NOW_S + 60)],
  ])('answers UNAUTHENTICATED for %s', async (_label, header) => {
    expect(await run(header)).toMatchObject({ ok: false, error: { code: 'UNAUTHENTICATED' } });
  });

  it('answers UNAUTHENTICATED when the session key cannot be read (fail closed)', async () => {
    const noKey = verifySession({
      secrets: fixedSecrets({}),
      clock: fakeClock(NOW),
      ownerId: 'kristoff',
    });

    expect(await noKey(bearer('kristoff', NOW_S + 60))).toMatchObject({
      ok: false,
      error: { code: 'UNAUTHENTICATED' },
    });
  });
});
