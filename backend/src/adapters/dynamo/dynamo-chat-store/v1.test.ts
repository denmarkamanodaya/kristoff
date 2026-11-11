import { describe, expect, it } from 'vitest';
import { aChat, aMessage } from '../../../../test/support/builders.js';
import type { DocClient } from '../_shared/dynamo-call.js';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { dynamoChatStore } from './v1.js';

const CHAT_ROW = {
  PK: 'OWNER#u1',
  SK: 'CHAT#cccccccccccccccc',
  title: 'New chat',
  docIds: [],
  createdAt: '2026-11-12T00:00:00.000Z',
  updatedAt: '2026-11-12T00:00:00.000Z',
  schemaVersion: 1,
  version: 1,
};
const MSG_ROW = {
  PK: 'CHAT#c',
  SK: 'MSG#01JAAAAAAAAAAAAAAAAAAAAAAA',
  ...aMessage(),
  schemaVersion: 1,
  version: 1,
};

describe('dynamo-chat-store v1', () => {
  it('creates a chat with its GSI1 keys', async () => {
    const { client, calls } = stubClient({});

    await dynamoChatStore({ client, table: 't' }).createChat('u1', aChat());

    expect(calls[0]?.input).toMatchObject({
      Item: {
        PK: 'OWNER#u1',
        SK: 'CHAT#cccccccccccccccc',
        GSI1PK: 'OWNER#u1#CHATS',
        GSI1SK: '2026-11-12T00:00:00.000Z#cccccccccccccccc',
        schemaVersion: 1,
        version: 1,
      },
      ConditionExpression: 'attribute_not_exists(PK)',
    });
  });

  it('lists from GSI1 newest first, and reads a chat back', async () => {
    const { client, calls } = stubClient({ Items: [CHAT_ROW] }, { Item: CHAT_ROW });
    const store = dynamoChatStore({ client, table: 't' });

    const page = await store.listChats('u1', undefined, 50);
    const chat = await store.getChat('u1', 'cccccccccccccccc');

    expect(page).toEqual({
      ok: true,
      value: {
        items: [
          {
            id: 'cccccccccccccccc',
            title: 'New chat',
            docIds: [],
            updatedAt: '2026-11-12T00:00:00.000Z',
            pinned: false,
            projectId: null,
          },
        ],
        nextCursor: null,
      },
    });
    expect(chat).toEqual({ ok: true, value: { value: aChat(), version: 1 } });
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      IndexName: 'GSI1',
      KeyConditionExpression: 'GSI1PK = :pk',
      ExpressionAttributeValues: { ':pk': 'OWNER#u1#CHATS' },
      ScanIndexForward: false,
      Limit: 50,
      ExclusiveStartKey: undefined,
    });
  });

  it('updates title, updatedAt and GSI1SK together, version-locked', async () => {
    const { client, calls } = stubClient({ Attributes: { ...CHAT_ROW, title: 'T', version: 2 } });

    await dynamoChatStore({ client, table: 't' }).updateChat(
      'u1',
      'cccccccccccccccc',
      { title: 'T', updatedAt: '2026-11-13T00:00:00.000Z' },
      1,
    );

    expect(calls[0]?.input).toMatchObject({
      UpdateExpression: 'SET #v = #v + :one, #f0 = :f0, #f4 = :f4, #f5 = :f5',
      ExpressionAttributeNames: {
        '#v': 'version',
        '#f0': 'title',
        '#f4': 'updatedAt',
        '#f5': 'GSI1SK',
      },
      ExpressionAttributeValues: {
        ':one': 1,
        ':expected': 1,
        ':f0': 'T',
        ':f4': '2026-11-13T00:00:00.000Z',
        ':f5': '2026-11-13T00:00:00.000Z#cccccccccccccccc',
      },
    });
  });

  it('puts a message under its chat and refuses one over 390 KB without calling DynamoDB', async () => {
    const { client, calls } = stubClient({});
    const store = dynamoChatStore({ client, table: 't' });

    await store.putMessage('c', aMessage());
    const big = await store.putMessage('c', aMessage({ text: 'x'.repeat(395_000) }));

    expect(calls).toHaveLength(1);
    expect(calls[0]?.input).toMatchObject({
      Item: { PK: 'CHAT#c', SK: 'MSG#01JAAAAAAAAAAAAAAAAAAAAAAA', role: 'user' },
    });
    expect(big).toMatchObject({ error: { code: 'INTERNAL' } });
  });

  it('reads the newest messages descending and returns them oldest first', async () => {
    const second = { ...MSG_ROW, SK: 'MSG#01JB', id: '01JBBBBBBBBBBBBBBBBBBBBBBB', text: 'second' };
    const { client, calls } = stubClient({ Items: [second, MSG_ROW] });

    const recent = await dynamoChatStore({ client, table: 't' }).recentMessages('c', 20);

    expect(recent.ok && recent.value.map((m) => m.text)).toEqual(['What is estoppel?', 'second']);
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': 'CHAT#c', ':sk': 'MSG#' },
      ScanIndexForward: false,
      Limit: 20,
      ConsistentRead: true,
    });
  });

  it('deletes messages page by page: projected Query, then one DeleteItem per message', async () => {
    const keys = [
      { PK: 'CHAT#c', SK: 'MSG#a' },
      { PK: 'CHAT#c', SK: 'MSG#b' },
    ];
    const { client, calls } = stubClient(
      { Items: keys.slice(0, 1), LastEvaluatedKey: keys[0] },
      {},
      { Items: keys.slice(1) },
      {},
    );

    const result = await dynamoChatStore({ client, table: 't' }).deleteMessages('c');

    expect(result).toEqual({ ok: true, value: 2 });
    expect(calls.map((c) => c.command)).toEqual([
      'QueryCommand',
      'DeleteCommand',
      'QueryCommand',
      'DeleteCommand',
    ]);
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': 'CHAT#c', ':sk': 'MSG#' },
      ProjectionExpression: 'PK, SK',
      Limit: 100,
      ConsistentRead: true,
      ExclusiveStartKey: undefined,
    });
    expect(calls[1]?.input).toEqual({ TableName: 't', Key: keys[0] });
    expect((calls[2]?.input as { ExclusiveStartKey: unknown }).ExclusiveStartKey).toEqual(keys[0]);
    expect(calls[3]?.input).toEqual({ TableName: 't', Key: keys[1] });
  });

  it('never has more than 8 DeleteItem calls in flight', async () => {
    const items = Array.from({ length: 30 }, (_, i) => ({ PK: 'CHAT#c', SK: `MSG#${String(i)}` }));
    let inFlight = 0;
    let peak = 0;
    let deletes = 0;
    const client = {
      send: async (command: { constructor: { name: string } }) => {
        if (command.constructor.name === 'QueryCommand') return { Items: items };
        inFlight += 1;
        deletes += 1;
        peak = Math.max(peak, inFlight);
        await new Promise((resolve) => setImmediate(resolve));
        inFlight -= 1;
        return {};
      },
    } as unknown as DocClient;

    const result = await dynamoChatStore({ client, table: 't' }).deleteMessages('c');

    expect(result).toEqual({ ok: true, value: 30 });
    expect([deletes, peak]).toEqual([30, 8]);
  });

  it('returns a failed delete as UPSTREAM_UNAVAILABLE, and deletes the chat item by its owner key', async () => {
    const { client, calls } = stubClient(
      { Items: [{ PK: 'CHAT#c', SK: 'MSG#a' }] },
      sdkError('ProvisionedThroughputExceededException'),
      {},
    );
    const store = dynamoChatStore({ client, table: 't' });

    expect(await store.deleteMessages('c')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE', details: { operation: 'dynamo.DeleteItem' } },
    });
    expect(await store.deleteChat('u1', 'cccccccccccccccc')).toEqual({
      ok: true,
      value: undefined,
    });
    expect(calls[2]).toEqual({
      command: 'DeleteCommand',
      input: { TableName: 't', Key: { PK: 'OWNER#u1', SK: 'CHAT#cccccccccccccccc' } },
    });
  });

  it('rejects a malformed projected key as INTERNAL without deleting', async () => {
    const { client, calls } = stubClient({ Items: [{ PK: 'CHAT#c' }] });

    expect(await dynamoChatStore({ client, table: 't' }).deleteMessages('c')).toMatchObject({
      error: { code: 'INTERNAL' },
    });
    expect(calls).toHaveLength(1);
  });

  it('clears projectId with a condition on the old project; a moved chat is ok', async () => {
    const { client, calls } = stubClient(
      {},
      sdkError('ConditionalCheckFailedException'),
      sdkError('InternalServerError'),
    );
    const store = dynamoChatStore({ client, table: 't' });

    expect((await store.clearProject('u1', 'cccccccccccccccc', 'pppppppppppppppp')).ok).toBe(true);
    expect((await store.clearProject('u1', 'cccccccccccccccc', 'pppppppppppppppp')).ok).toBe(true);
    expect(await store.clearProject('u1', 'cccccccccccccccc', 'pppppppppppppppp')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      Key: { PK: 'OWNER#u1', SK: 'CHAT#cccccccccccccccc' },
      UpdateExpression: 'SET projectId = :none, #v = #v + :one',
      ConditionExpression: 'projectId = :pid',
      ExpressionAttributeNames: { '#v': 'version' },
      ExpressionAttributeValues: { ':none': null, ':one': 1, ':pid': 'pppppppppppppppp' },
    });
  });
});
