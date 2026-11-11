import type { APIGatewayRequestAuthorizerEventV2 } from 'aws-lambda';
import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { httpDeps } from '../../../test/support/http-events.js';
import { createAuthorizer } from './v1.js';

const event = (headers: Record<string, string> | null) =>
  ({ headers }) as unknown as APIGatewayRequestAuthorizerEventV2;

describe('authorizer v1', () => {
  it('allows a valid session and passes sub in the context', async () => {
    const seen: unknown[] = [];
    const authorize = createAuthorizer({
      log: httpDeps().log,
      verifySession: (h) => (seen.push(h), Promise.resolve(ok({ uid: 'kristoff' }))),
    });

    const result = await authorize(event({ authorization: 'Bearer a.b' }));

    expect([result, seen]).toEqual([
      { isAuthorized: true, context: { sub: 'kristoff' } },
      ['Bearer a.b'],
    ]);
  });

  it('denies and logs the code when the session does not verify, including with no headers', async () => {
    const deps = httpDeps();
    const authorize = createAuthorizer({
      log: deps.log,
      verifySession: () => Promise.resolve(err(appError('UNAUTHENTICATED', 'no'))),
    });

    const result = await authorize(event(null));

    expect(result.isAuthorized).toBe(false);
    expect(deps.lines).toEqual([
      { level: 'warn', event: 'auth.denied', operation: 'authorizer', code: 'UNAUTHENTICATED' },
    ]);
  });
});
