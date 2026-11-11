import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { dynamoUsageMeter } from './v1.js';

describe('dynamo-usage-meter v1', () => {
  it('admits with one conditional ADD on the day item; a failed condition is DAILY_CAP_REACHED', async () => {
    const { client, calls } = stubClient({}, sdkError('ConditionalCheckFailedException'));
    const meter = dynamoUsageMeter({ client, table: 't' });

    expect((await meter.admitQuestion('u1', '2026-11-12', 50)).ok).toBe(true);
    expect(await meter.admitQuestion('u1', '2026-11-12', 50)).toMatchObject({
      error: { code: 'DAILY_CAP_REACHED' },
    });
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      Key: { PK: 'OWNER#u1', SK: 'USAGE#2026-11-12' },
      UpdateExpression: 'ADD questions :one, #v :one SET schemaVersion = :sv',
      ConditionExpression: 'attribute_not_exists(questions) OR questions < :cap',
      ExpressionAttributeNames: { '#v': 'version' },
      ExpressionAttributeValues: { ':one': 1, ':cap': 50, ':sv': 1 },
    });
  });

  it('adds tokens to the day and the total items', async () => {
    const { client, calls } = stubClient({}, {});

    await dynamoUsageMeter({ client, table: 't' }).addTokens('u1', '2026-11-12', {
      input: 1,
      output: 2,
      cacheRead: 3,
      cacheWrite: 4,
    });

    expect(calls.map((c) => (c.input as { Key: unknown }).Key)).toEqual([
      { PK: 'OWNER#u1', SK: 'USAGE#2026-11-12' },
      { PK: 'OWNER#u1', SK: 'USAGE#TOTAL' },
    ]);
    expect(calls[0]?.input).toMatchObject({
      ExpressionAttributeValues: { ':in': 1, ':out': 2, ':cr': 3, ':cw': 4 },
    });
  });

  it('reads both counters, zero when missing', async () => {
    const { client } = stubClient({
      Responses: {
        t: [{ SK: 'USAGE#TOTAL', input: 5, output: 6, cacheRead: 7, cacheWrite: 8, questions: 3 }],
      },
    });

    expect(await dynamoUsageMeter({ client, table: 't' }).read('u1', '2026-11-12')).toEqual({
      ok: true,
      value: {
        today: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        total: { input: 5, output: 6, cacheRead: 7, cacheWrite: 8 },
      },
    });
  });

  it('passes a failed token write through', async () => {
    const { client } = stubClient({}, sdkError('InternalServerError'));

    expect(
      await dynamoUsageMeter({ client, table: 't' }).addTokens('u1', 'd', {
        input: 0,
        output: 0,
        cacheRead: 0,
        cacheWrite: 0,
      }),
    ).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });
});
