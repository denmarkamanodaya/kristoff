import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { dynamoAuthStateStore } from './v1.js';

describe('dynamo-auth-state-store v1', () => {
  it('reads the initial state at version 0 when no item exists, else the stored row', async () => {
    const row = {
      PK: 'AUTH#u1',
      SK: 'TOTP',
      lastStep: 9,
      failCount: 2,
      lockedUntil: 0,
      schemaVersion: 1,
      version: 4,
    };
    const { client } = stubClient({}, { Item: row });
    const store = dynamoAuthStateStore({ client, table: 't' });

    expect(await store.get('u1')).toEqual({
      ok: true,
      value: { value: { lastStep: 0, failCount: 0, lockedUntil: 0 }, version: 0 },
    });
    expect(await store.get('u1')).toEqual({
      ok: true,
      value: { value: { lastStep: 9, failCount: 2, lockedUntil: 0 }, version: 4 },
    });
  });

  it('accepts a step with the replay condition lastStep < :step; a failed condition is OTP_REPLAYED', async () => {
    const { client, calls } = stubClient({}, sdkError('ConditionalCheckFailedException'));
    const store = dynamoAuthStateStore({ client, table: 't' });

    expect((await store.acceptStep('u1', 100)).ok).toBe(true);
    expect(await store.acceptStep('u1', 100)).toMatchObject({ error: { code: 'OTP_REPLAYED' } });
    expect(calls[0]?.input).toMatchObject({
      Key: { PK: 'AUTH#u1', SK: 'TOTP' },
      ConditionExpression: 'attribute_not_exists(lastStep) OR lastStep < :step',
      ExpressionAttributeValues: { ':step': 100, ':zero': 0, ':one': 1, ':sv': 1 },
    });
  });

  it('writes a failure as a new item at version 0, else conditional on the read version', async () => {
    const { client, calls } = stubClient({}, sdkError('ConditionalCheckFailedException'));
    const store = dynamoAuthStateStore({ client, table: 't' });
    const next = { lastStep: 0, failCount: 1, lockedUntil: 0 };

    await store.recordFailure('u1', next, 0);
    const conflict = await store.recordFailure('u1', next, 3);

    expect(calls.map((c) => c.input)).toEqual([
      {
        TableName: 't',
        Item: { PK: 'AUTH#u1', SK: 'TOTP', ...next, schemaVersion: 1, version: 1 },
        ConditionExpression: 'attribute_not_exists(PK)',
      },
      {
        TableName: 't',
        Item: { PK: 'AUTH#u1', SK: 'TOTP', ...next, schemaVersion: 1, version: 4 },
        ConditionExpression: '#v = :expected',
        ExpressionAttributeNames: { '#v': 'version' },
        ExpressionAttributeValues: { ':expected': 3 },
      },
    ]);
    expect(conflict).toMatchObject({ error: { code: 'VERSION_CONFLICT' } });
  });
});
