import { describe, expect, it } from 'vitest';
import { hotp, stepAt } from '../../domain/auth/verify-totp/index.js';
import { verifySessionToken } from '../../domain/auth/session-token/index.js';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { fixedSecrets } from '../../../test/support/fakes/fixed-ports.js';
import { memoryAuthStateStore } from '../../../test/support/fakes/memory-auth-state-store.js';
import { verifyOtp } from './v1.js';

// RFC 6238 secret "12345678901234567890" in base32; session key = 32 bytes of 7 in base64.
const TOTP = 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ';
const RAW = Buffer.from('12345678901234567890');
const KEY = Buffer.alloc(32, 7);

const setup = (
  secrets = fixedSecrets({ 'totp-secret': TOTP, 'session-key': KEY.toString('base64') }),
) => {
  const clock = fakeClock(Date.UTC(2026, 10, 12, 1, 0, 0));
  const auth = memoryAuthStateStore();
  const run = verifyOtp({
    authState: auth.store,
    secrets,
    clock,
    ownerId: 'kristoff',
    sessionTtlSeconds: 43200,
  });
  const codeNow = () => hotp(RAW, stepAt(clock.now()));
  return { clock, auth, run, codeNow };
};

describe('verify-otp v1', () => {
  it('issues a session for the owner that expires after the configured lifetime', async () => {
    const { run, codeNow, clock } = setup();

    const result = await run(codeNow());

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const nowSeconds = Math.floor(clock.now() / 1000);
    expect(verifySessionToken(KEY, result.value.token, nowSeconds)).toEqual({
      sub: 'kristoff',
      iat: nowSeconds,
      exp: nowSeconds + 43200,
    });
    expect(result.value.expiresAt).toBe('2026-11-12T13:00:00.000Z');
  });

  it('refuses the same code a second time (replay)', async () => {
    const { run, codeNow } = setup();
    const code = codeNow();
    await run(code);

    const again = await run(code);

    expect(again).toMatchObject({ ok: false, error: { code: 'OTP_REPLAYED' } });
  });

  it('refuses an older code after a newer one was used', async () => {
    const { run, clock } = setup();
    const step = stepAt(clock.now());
    await run(hotp(RAW, step));

    const older = await run(hotp(RAW, step - 1));

    expect(older).toMatchObject({ ok: false, error: { code: 'OTP_REPLAYED' } });
  });

  it('answers OTP_INVALID for a wrong code and counts the failure', async () => {
    const { run, auth } = setup();

    const result = await run('000000');

    expect(result).toMatchObject({ ok: false, error: { code: 'OTP_INVALID' } });
    expect(auth.rows.get('kristoff')?.state.failCount).toBe(1);
  });

  it('locks on the fifth wrong code with Retry-After 900 s, then refuses even the right code', async () => {
    const { run, codeNow } = setup();
    for (let i = 0; i < 4; i += 1) await run('000000');

    const fifth = await run('000000');
    const right = await run(codeNow());

    expect(fifth).toMatchObject({
      ok: false,
      error: { code: 'OTP_LOCKED', details: { retryAfterSeconds: 900 } },
    });
    expect(right).toMatchObject({ ok: false, error: { code: 'OTP_LOCKED' } });
  });

  it('accepts the right code again once the lock has passed, and resets the count', async () => {
    const { run, codeNow, clock, auth } = setup();
    for (let i = 0; i < 5; i += 1) await run('000000');
    clock.advance(15 * 60 * 1000);

    const result = await run(codeNow());

    expect(result.ok).toBe(true);
    expect(auth.rows.get('kristoff')?.state).toMatchObject({ failCount: 0, lockedUntil: 0 });
  });

  it('fails closed when the TOTP secret is missing or not base32', async () => {
    const missing = setup(fixedSecrets({ 'session-key': 'a2V5' }));
    const garbage = setup(fixedSecrets({ 'totp-secret': '!!!', 'session-key': 'a2V5' }));

    expect(await missing.run('123456')).toMatchObject({ ok: false, error: { code: 'INTERNAL' } });
    expect(await garbage.run('123456')).toMatchObject({ ok: false, error: { code: 'INTERNAL' } });
  });

  it('fails closed when the session key is missing after a right code', async () => {
    const { run, codeNow } = setup(fixedSecrets({ 'totp-secret': TOTP }));

    expect(await run(codeNow())).toMatchObject({ ok: false, error: { code: 'INTERNAL' } });
  });

  it('passes a state-store failure through', async () => {
    const { auth, clock } = setup();
    const run = verifyOtp({
      authState: {
        ...auth.store,
        get: () => Promise.resolve({ ok: false, error: new Error('x') as never }),
      },
      secrets: fixedSecrets({}),
      clock,
      ownerId: 'kristoff',
      sessionTtlSeconds: 1,
    });

    expect((await run('123456')).ok).toBe(false);
  });
});
