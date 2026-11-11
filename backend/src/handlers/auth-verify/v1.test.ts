import { describe, expect, it } from 'vitest';
import { SessionResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createAuthVerifyHandler } from './v1.js';

const ROUTE = 'POST /v1/auth/verify';
const SESSION = { token: 'p.m', expiresAt: '2026-11-12T13:00:00.000Z' };

describe('auth-verify handler v1', () => {
  it('passes the code to verify-otp without needing an authorizer, and answers the session', async () => {
    const codes: string[] = [];
    const handler = createAuthVerifyHandler({
      http: httpDeps(),
      verifyOtp: (code) => (codes.push(code), Promise.resolve(ok(SESSION))),
    });

    const res = await handler(
      httpEvent({ routeKey: ROUTE, body: { code: '123456' }, sub: undefined }),
    );

    expect([res.statusCode, SessionResponse.parse(bodyOf(res)), codes]).toEqual([
      200,
      SESSION,
      ['123456'],
    ]);
  });

  it.each([
    ['5 digits', { code: '12345' }],
    ['letters', { code: '12345a' }],
    ['an extra field', { code: '123456', x: 1 }],
  ])('answers 400 VALIDATION_FAILED for %s without calling verify-otp', async (_label, body) => {
    const handler = createAuthVerifyHandler({
      http: httpDeps(),
      verifyOtp: () => Promise.reject(new Error('called')),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, body, sub: undefined }));

    expect([res.statusCode, bodyOf(res)]).toMatchObject([
      400,
      { error: { code: 'VALIDATION_FAILED' } },
    ]);
  });

  it('answers 429 OTP_LOCKED with the minutes-left Retry-After', async () => {
    const locked = appError('OTP_LOCKED', 'wait', { retryAfterSeconds: 840 });
    const handler = createAuthVerifyHandler({
      http: httpDeps(),
      verifyOtp: () => Promise.resolve(err(locked)),
    });

    const res = await handler(
      httpEvent({ routeKey: ROUTE, body: { code: '000000' }, sub: undefined }),
    );

    expect([res.statusCode, res.headers?.['retry-after']]).toEqual([429, '840']);
  });
});
