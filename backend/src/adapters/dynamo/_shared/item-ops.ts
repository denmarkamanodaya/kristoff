// The item operations both stores share (core/dry.md placement: adapters/<service>/_shared):
// create-if-absent, consistent get, version-locked update, unconditional delete and a bounded
// query, each guarded.
// Contract: docs/contract.md "Storage". Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import {
  DeleteCommand,
  GetCommand,
  PutCommand,
  QueryCommand,
  UpdateCommand,
  type QueryCommandInput,
} from '@aws-sdk/lib-dynamodb';
import type { ItemKey } from '../../../domain/keys/index.js';
import type { AppError } from '../../../shared/errors/app-error.js';
import { ok, type Result } from '../../../shared/result/result.js';
import { guarded, versionedUpdate, type DynamoTarget } from './dynamo-call.js';

/**
 * Writes a new item; never overwrites.
 * @param target - Client and table.
 * @param item - The full item, keys included.
 * @param onExists - The error an existing key means.
 * @returns Ok, `onExists()` or `UPSTREAM_UNAVAILABLE`.
 */
export const putNew = async (
  { client, table }: DynamoTarget,
  item: Record<string, unknown>,
  onExists: () => AppError,
): Promise<Result<void, AppError>> => {
  const put = await guarded(
    () =>
      client.send(
        new PutCommand({
          TableName: table,
          Item: item,
          ConditionExpression: 'attribute_not_exists(PK)',
        }),
      ),
    'dynamo.PutItem',
    onExists,
  );
  return put.ok ? ok(undefined) : put;
};

/**
 * Reads one item with a consistent read.
 * @param target - Client and table.
 * @param key - Primary key.
 * @returns The raw item, `undefined` when absent, or `UPSTREAM_UNAVAILABLE`.
 */
export const getItem = async (
  { client, table }: DynamoTarget,
  key: ItemKey,
): Promise<Result<Record<string, unknown> | undefined, AppError>> => {
  const got = await guarded(
    () => client.send(new GetCommand({ TableName: table, Key: key, ConsistentRead: true })),
    'dynamo.GetItem',
  );
  return got.ok ? ok(got.value.Item) : got;
};

/**
 * Sets fields if the stored version equals `version`, bumping it.
 * @param target - Client and table.
 * @param key - Primary key.
 * @param fields - Attribute values (`undefined` = unchanged).
 * @param change - The version read and the error a lost race means.
 * @returns The item after the update, `change.onConflict()` or `UPSTREAM_UNAVAILABLE`.
 */
export const updateVersioned = async (
  { client, table }: DynamoTarget,
  key: ItemKey,
  fields: Readonly<Record<string, unknown>>,
  change: { readonly version: number; readonly onConflict: () => AppError },
): Promise<Result<Record<string, unknown> | undefined, AppError>> => {
  const updated = await guarded(
    () =>
      client.send(
        new UpdateCommand({
          TableName: table,
          Key: key,
          ...versionedUpdate(fields, change.version),
          ReturnValues: 'ALL_NEW',
        }),
      ),
    'dynamo.UpdateItem',
    change.onConflict,
  );
  return updated.ok ? ok(updated.value.Attributes) : updated;
};

/**
 * Runs one bounded query page.
 * @param target - Client and table.
 * @param input - Key condition, values, limit, start key and direction.
 * @returns The items and `LastEvaluatedKey`, or `UPSTREAM_UNAVAILABLE`.
 */
export const queryPage = async (
  { client, table }: DynamoTarget,
  input: Omit<QueryCommandInput, 'TableName'>,
): Promise<
  Result<{ items: unknown[]; lastKey: Record<string, unknown> | undefined }, AppError>
> => {
  const page = await guarded(
    () => client.send(new QueryCommand({ TableName: table, ...input })),
    'dynamo.Query',
  );
  return page.ok
    ? ok({ items: page.value.Items ?? [], lastKey: page.value.LastEvaluatedKey })
    : page;
};

/**
 * Deletes one item; deleting an absent item is not an error (idempotent).
 * @param target - Client and table.
 * @param key - Primary key.
 * @returns Ok or `UPSTREAM_UNAVAILABLE`.
 */
export const deleteItem = async (
  { client, table }: DynamoTarget,
  key: ItemKey,
): Promise<Result<void, AppError>> => {
  const removed = await guarded(
    () => client.send(new DeleteCommand({ TableName: table, Key: key })),
    'dynamo.DeleteItem',
  );
  return removed.ok ? ok(undefined) : removed;
};
