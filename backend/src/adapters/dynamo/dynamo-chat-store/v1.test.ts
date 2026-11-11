import { describe, expect, it } from 'vitest';
import { aChat, aMessage } from '../../../../test/support/builders.js';
import { stubClient } from '../../../../test/support/sdk-stub.js';
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
      UpdateExpression: 'SET #v = #v + :one, #f0 = :f0, #f2 = :f2, #f3 = :f3',
      ExpressionAttributeNames: {
        '#v': 'version',
        '#f0': 'title',
        '#f2': 'updatedAt',
        '#f3': 'GSI1SK',
      },
      ExpressionAttributeValues: {
        ':one': 1,
        ':expected': 1,
        ':f0': 'T',
        ':f2': '2026-11-13T00:00:00.000Z',
        ':f3': '2026-11-13T00:00:00.000Z#cccccccccccccccc',
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
});
