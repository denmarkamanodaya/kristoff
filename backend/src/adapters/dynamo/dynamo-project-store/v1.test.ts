import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { dynamoProjectStore } from './v1.js';

const P = { id: 'pppppppppppppppp', name: 'Torts', createdAt: 'a', updatedAt: 'a' };
const ROW = {
  PK: 'OWNER#u1',
  SK: 'PROJECT#pppppppppppppppp',
  name: 'Torts',
  createdAt: 'a',
  updatedAt: 'a',
  schemaVersion: 1,
  version: 2,
};
const cancelled = (...codes: string[]) =>
  Object.assign(sdkError('TransactionCanceledException'), {
    CancellationReasons: codes.map((Code) => ({ Code })),
  });

describe('dynamo-project-store v1', () => {
  it('creates in one transaction: counter ADD while n < cap, then put-if-absent', async () => {
    const { client, calls } = stubClient({});

    expect(await dynamoProjectStore({ client, table: 't' }).create('u1', P, 50)).toEqual({
      ok: true,
      value: undefined,
    });
    expect(calls[0]).toEqual({
      command: 'TransactWriteCommand',
      input: {
        TransactItems: [
          {
            Update: {
              TableName: 't',
              Key: { PK: 'OWNER#u1', SK: 'COUNT#PROJECTS' },
              UpdateExpression: 'ADD n :one SET schemaVersion = :sv',
              ConditionExpression: 'attribute_not_exists(n) OR n < :cap',
              ExpressionAttributeValues: { ':one': 1, ':cap': 50, ':sv': 1 },
            },
          },
          {
            Put: {
              TableName: 't',
              Item: {
                PK: 'OWNER#u1',
                SK: 'PROJECT#pppppppppppppppp',
                name: 'Torts',
                createdAt: 'a',
                updatedAt: 'a',
                schemaVersion: 1,
                version: 1,
              },
              ConditionExpression: 'attribute_not_exists(PK)',
            },
          },
        ],
      },
    });
  });

  it('maps a cancelled counter condition to PROJECT_LIMIT_REACHED, anything else to UPSTREAM_UNAVAILABLE', async () => {
    const { client } = stubClient(
      cancelled('ConditionalCheckFailed', 'None'),
      cancelled('None', 'ConditionalCheckFailed'),
      sdkError('InternalServerError'),
    );
    const store = dynamoProjectStore({ client, table: 't' });

    expect(await store.create('u1', P, 50)).toMatchObject({
      error: { code: 'PROJECT_LIMIT_REACHED' },
    });
    expect(await store.create('u1', P, 50)).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
    expect(await store.create('u1', P, 50)).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE', details: { operation: 'dynamo.TransactWriteItems' } },
    });
  });

  it('deletes and decrements in one transaction; a missing project is PROJECT_NOT_FOUND', async () => {
    const { client, calls } = stubClient({}, cancelled('ConditionalCheckFailed', 'None'));
    const store = dynamoProjectStore({ client, table: 't' });

    expect((await store.delete('u1', P.id)).ok).toBe(true);
    expect(await store.delete('u1', P.id)).toMatchObject({ error: { code: 'PROJECT_NOT_FOUND' } });
    expect(calls[0]?.input).toEqual({
      TransactItems: [
        {
          Delete: {
            TableName: 't',
            Key: { PK: 'OWNER#u1', SK: 'PROJECT#pppppppppppppppp' },
            ConditionExpression: 'attribute_exists(PK)',
          },
        },
        {
          Update: {
            TableName: 't',
            Key: { PK: 'OWNER#u1', SK: 'COUNT#PROJECTS' },
            UpdateExpression: 'ADD n :minus',
            ConditionExpression: 'n > :zero',
            ExpressionAttributeValues: { ':minus': -1, ':zero': 0 },
          },
        },
      ],
    });
  });

  it('gets, lists (bounded by the cap) and renames with a version lock', async () => {
    const { client, calls } = stubClient(
      { Item: ROW },
      {},
      { Items: [ROW] },
      { Attributes: { ...ROW, name: 'New', version: 3 } },
    );
    const store = dynamoProjectStore({ client, table: 't' });

    expect(await store.get('u1', P.id)).toEqual({ ok: true, value: { value: P, version: 2 } });
    expect(await store.get('u1', P.id)).toMatchObject({ error: { code: 'PROJECT_NOT_FOUND' } });
    expect(await store.list('u1')).toEqual({ ok: true, value: [P] });
    expect(await store.rename('u1', P.id, { name: 'New', updatedAt: 'a', version: 2 })).toEqual({
      ok: true,
      value: { ...P, name: 'New' },
    });
    expect(calls[2]?.input).toEqual({
      TableName: 't',
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': 'OWNER#u1', ':sk': 'PROJECT#' },
      Limit: 50,
    });
    expect(calls[3]?.input).toMatchObject({
      ConditionExpression: '#v = :expected',
      ExpressionAttributeValues: { ':expected': 2 },
    });
  });
});
