import { describe, expect, it } from 'vitest';
import { z } from 'zod';
import { appError } from '../errors/app-error.js';
import { err, ok } from '../result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { errorReply, parseJson, runPublicRoute, runRoute } from './http.js';

describe('http plumbing', () => {
  it('passes the authorizer sub as the principal and logs one hashed outcome line', async () => {
    const deps = httpDeps();
    const seen: unknown[] = [];

    const res = await runRoute(deps, 'docs-list', httpEvent({ routeKey: 'GET /v1/docs' }), (p) => {
      seen.push(p);
      return Promise.resolve(ok({ body: { a: 1 } }));
    });

    expect([res.statusCode, bodyOf(res), res.headers?.['x-request-id']]).toEqual([
      200,
      { a: 1 },
      'req-1',
    ]);
    expect(seen).toEqual([{ uid: 'kristoff' }]);
    expect(deps.lines).toEqual([
      {
        level: 'info',
        event: 'http.request',
        operation: 'docs-list',
        requestId: 'req-1',
        outcome: 'ok',
        'http.route': 'GET /v1/docs',
        'http.response.status_code': 200,
        durationMs: 0,
        principalHash: expect.stringMatching(/^[a-f0-9]{16}$/) as unknown,
      },
    ]);
  });

  it.each([
    ['no authorizer context', httpEvent({ routeKey: 'GET /v1/docs', sub: undefined })],
    ['a malformed sub', httpEvent({ routeKey: 'GET /v1/docs', sub: 'a b' })],
    ['not an object', 'garbage' as never],
  ])('answers 401 UNAUTHENTICATED without calling the route for %s', async (_label, event) => {
    const calls: unknown[] = [];

    const res = await runRoute(httpDeps(), 'x', event, () => {
      calls.push(1);
      return Promise.resolve(ok({ body: {} }));
    });

    expect([res.statusCode, calls]).toEqual([401, []]);
    expect(bodyOf(res)).toMatchObject({ error: { code: 'UNAUTHENTICATED' } });
  });

  it('runs the public route without a principal', async () => {
    const res = await runPublicRoute(
      httpDeps(),
      'auth-verify',
      httpEvent({ routeKey: 'POST /v1/auth/verify', sub: undefined }),
      () => Promise.resolve(ok({ body: { t: 1 } })),
    );

    expect(res.statusCode).toBe(200);
  });

  it('maps codes to the contract status, with the per-error Retry-After override', () => {
    const locked = errorReply(appError('OTP_LOCKED', 'wait', { retryAfterSeconds: 42 }), 'r');
    const disabled = errorReply(appError('AI_DISABLED', 'off'), 'r');
    const missing = errorReply(appError('DOC_NOT_FOUND', 'no'), 'r');

    expect([locked.status, locked.headers['retry-after']]).toEqual([429, '42']);
    expect([disabled.status, disabled.headers['retry-after']]).toEqual([503, '300']);
    expect([missing.status, missing.headers['retry-after']]).toEqual([404, undefined]);
    expect(missing.envelope).toEqual({
      error: { code: 'DOC_NOT_FOUND', message: 'no', requestId: 'r' },
    });
  });

  it('sends an internal-only code as 500 INTERNAL with a generic message', () => {
    const reply = errorReply(appError('VERSION_CONFLICT', 'secret detail'), 'r');

    expect([reply.status, reply.envelope]).toEqual([
      500,
      { error: { code: 'INTERNAL', message: 'internal error', requestId: 'r' } },
    ]);
  });

  it('rejects a body that is too large, not JSON, or off-contract, and decodes base64', () => {
    const schema = z.object({ a: z.number().min(0).max(9) }).strict();

    const big = z.object({ pad: z.string().max(100_000) }).strict();
    expect(parseJson(JSON.stringify({ pad: 'x'.repeat(66_000) }), false, big)).toMatchObject({
      error: { code: 'VALIDATION_FAILED', message: 'request body too large' },
    });
    expect(parseJson(JSON.stringify({ pad: 'x'.repeat(60_000) }), false, big).ok).toBe(true);
    expect(parseJson('{', false, schema)).toMatchObject({ error: { code: 'VALIDATION_FAILED' } });
    expect(parseJson('{"a":1,"b":2}', false, schema)).toMatchObject({
      error: { code: 'VALIDATION_FAILED' },
    });
    expect(parseJson(undefined, false, schema)).toMatchObject({ ok: false });
    expect(parseJson(Buffer.from('{"a":1}').toString('base64'), true, schema)).toEqual({
      ok: true,
      value: { a: 1 },
    });
  });

  it('answers the error envelope when the route fails', async () => {
    const res = await runRoute(httpDeps(), 'x', httpEvent({ routeKey: 'GET /v1/docs' }), () =>
      Promise.resolve(err(appError('DAILY_CAP_REACHED', 'cap'))),
    );

    expect([res.statusCode, res.headers?.['retry-after']]).toEqual([429, '3600']);
  });
});
