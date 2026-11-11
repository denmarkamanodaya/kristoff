import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../../test/support/builders.js';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { dynamoDocStore } from './v1.js';

const ROW = {
  PK: 'OWNER#u1',
  SK: 'DOC#aaaaaaaaaaaaaaaa',
  title: 'People v. Cruz',
  size: 1234,
  status: 'ready',
  createdAt: '2026-11-12T00:00:00.000Z',
  s3Key: 'k',
  pageCount: 10,
  parts: [],
  schemaVersion: 1,
  version: 3,
};

describe('dynamo-doc-store v1', () => {
  it('creates with attribute_not_exists, the s3 key, schemaVersion 1 and version 1', async () => {
    const { client, calls } = stubClient({});

    await dynamoDocStore({ client, table: 't' }).create('u1', aDoc(), 'docs/u1/a.pdf');

    expect(calls).toEqual([
      {
        command: 'PutCommand',
        input: {
          TableName: 't',
          Item: {
            PK: 'OWNER#u1',
            SK: 'DOC#aaaaaaaaaaaaaaaa',
            title: 'People v. Cruz',
            size: 1234,
            status: 'ready',
            createdAt: '2026-11-12T00:00:00.000Z',
            pageCount: 10,
            parts: [],
            s3Key: 'docs/u1/a.pdf',
            schemaVersion: 1,
            version: 1,
          },
          ConditionExpression: 'attribute_not_exists(PK)',
        },
      },
    ]);
  });

  it('reads consistently and maps the row, DOC_NOT_FOUND when absent, INTERNAL when malformed', async () => {
    const { client, calls } = stubClient({ Item: ROW }, {}, { Item: { ...ROW, size: 'big' } });
    const store = dynamoDocStore({ client, table: 't' });

    expect(await store.get('u1', 'aaaaaaaaaaaaaaaa')).toEqual({
      ok: true,
      value: { value: aDoc(), version: 3 },
    });
    expect(await store.get('u1', 'aaaaaaaaaaaaaaaa')).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
    expect(await store.get('u1', 'aaaaaaaaaaaaaaaa')).toMatchObject({
      error: { code: 'INTERNAL' },
    });
    expect(calls[0]).toEqual({
      command: 'GetCommand',
      input: {
        TableName: 't',
        Key: { PK: 'OWNER#u1', SK: 'DOC#aaaaaaaaaaaaaaaa' },
        ConsistentRead: true,
      },
    });
  });

  it('updates with a version condition and SET of only the given fields', async () => {
    const { client, calls } = stubClient(
      { Attributes: { ...ROW, version: 4 } },
      sdkError('ConditionalCheckFailedException'),
    );
    const store = dynamoDocStore({ client, table: 't' });

    expect(await store.update('u1', 'aaaaaaaaaaaaaaaa', { status: 'ready' }, 3)).toEqual({
      ok: true,
      value: aDoc(),
    });
    expect(await store.update('u1', 'aaaaaaaaaaaaaaaa', { status: 'ready' }, 3)).toMatchObject({
      error: { code: 'VERSION_CONFLICT' },
    });
    expect(calls[0]?.input).toEqual({
      TableName: 't',
      Key: { PK: 'OWNER#u1', SK: 'DOC#aaaaaaaaaaaaaaaa' },
      UpdateExpression: 'SET #v = #v + :one, #f1 = :f1',
      ConditionExpression: '#v = :expected',
      ExpressionAttributeNames: { '#v': 'version', '#f1': 'status' },
      ExpressionAttributeValues: { ':one': 1, ':expected': 3, ':f1': 'ready' },
      ReturnValues: 'ALL_NEW',
    });
  });

  it('queries one bounded page under the owner and round-trips the cursor', async () => {
    const lastKey = { PK: 'OWNER#u1', SK: 'DOC#aaaaaaaaaaaaaaaa' };
    const { client, calls } = stubClient(
      { Items: [ROW], LastEvaluatedKey: lastKey },
      { Items: [] },
    );
    const store = dynamoDocStore({ client, table: 't' });

    const first = await store.list('u1', undefined, 100);
    const cursor = first.ok ? (first.value.nextCursor ?? undefined) : undefined;
    await store.list('u1', cursor, 100);

    expect(first).toMatchObject({ ok: true, value: { items: [aDoc()] } });
    expect(calls.map((c) => c.input)).toEqual([
      {
        TableName: 't',
        KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
        ExpressionAttributeValues: { ':pk': 'OWNER#u1', ':sk': 'DOC#' },
        ScanIndexForward: false,
        Limit: 100,
        ExclusiveStartKey: undefined,
      },
      {
        TableName: 't',
        KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
        ExpressionAttributeValues: { ':pk': 'OWNER#u1', ':sk': 'DOC#' },
        ScanIndexForward: false,
        Limit: 100,
        ExclusiveStartKey: lastKey,
      },
    ]);
  });

  it("rejects another owner's cursor without querying", async () => {
    const foreign = Buffer.from(JSON.stringify({ PK: 'OWNER#eve', SK: 'DOC#x' })).toString(
      'base64url',
    );
    const { client, calls } = stubClient();

    expect(await dynamoDocStore({ client, table: 't' }).list('u1', foreign, 100)).toMatchObject({
      error: { code: 'VALIDATION_FAILED' },
    });
    expect(calls).toEqual([]);
  });

  it('batch-reads consistently; unprocessed keys are UPSTREAM_UNAVAILABLE; none means no call', async () => {
    const { client, calls } = stubClient(
      { Responses: { t: [ROW] } },
      { Responses: { t: [] }, UnprocessedKeys: { t: {} } },
    );
    const store = dynamoDocStore({ client, table: 't' });

    expect(await store.getMany('u1', ['aaaaaaaaaaaaaaaa'])).toEqual({ ok: true, value: [aDoc()] });
    expect(await store.getMany('u1', ['aaaaaaaaaaaaaaaa'])).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
    expect(await store.getMany('u1', [])).toEqual({ ok: true, value: [] });
    expect(calls[0]?.input).toEqual({
      RequestItems: {
        t: { Keys: [{ PK: 'OWNER#u1', SK: 'DOC#aaaaaaaaaaaaaaaa' }], ConsistentRead: true },
      },
    });
    expect(calls).toHaveLength(2);
  });

  it('maps any other SDK failure to UPSTREAM_UNAVAILABLE naming the operation', async () => {
    const { client } = stubClient(sdkError('ProvisionedThroughputExceededException'));

    expect(await dynamoDocStore({ client, table: 't' }).get('u1', 'a')).toMatchObject({
      error: {
        code: 'UPSTREAM_UNAVAILABLE',
        details: {
          operation: 'dynamo.GetItem',
          'error.type': 'ProvisionedThroughputExceededException',
        },
      },
    });
  });

  it('removes the doc item by its owner key', async () => {
    const { client, calls } = stubClient({});

    expect(await dynamoDocStore({ client, table: 't' }).remove('u1', 'aaaaaaaaaaaaaaaa')).toEqual({
      ok: true,
      value: undefined,
    });
    expect(calls[0]).toEqual({
      command: 'DeleteCommand',
      input: { TableName: 't', Key: { PK: 'OWNER#u1', SK: 'DOC#aaaaaaaaaaaaaaaa' } },
    });
  });

  it('reads a doc written before v2 (no pageCount, no parts) with null and []', async () => {
    const legacy = {
      PK: 'OWNER#u1',
      SK: 'DOC#aaaaaaaaaaaaaaaa',
      title: 'T',
      size: 1,
      status: 'ready',
      createdAt: 'a',
      s3Key: 'k',
      schemaVersion: 1,
      version: 1,
    };
    const { client } = stubClient({ Item: legacy });

    expect(
      await dynamoDocStore({ client, table: 't' }).get('u1', 'aaaaaaaaaaaaaaaa'),
    ).toMatchObject({
      ok: true,
      value: { value: { pageCount: null, parts: [] } },
    });
  });
});
