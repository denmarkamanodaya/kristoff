// DynamoDB adapter plumbing (core/dry.md placement: adapters/<service>/_shared): one guarded send
// that translates SDK exceptions to typed errors, the opaque cursor codec, and the item envelope
// (schemaVersion + version) every item carries. Contract: docs/contract.md "Storage".
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { DynamoDBDocumentClient } from '@aws-sdk/lib-dynamodb';
import { MAX_CURSOR } from '../../../domain/limits.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { errorTypeOf, upstreamError } from '../../../shared/errors/vendor-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';

/** The slice of the document client the adapters use (a stub in tests). */
export type DocClient = Pick<DynamoDBDocumentClient, 'send'>;

/** Table name plus client, given by the composition root. */
export interface DynamoTarget {
  readonly client: DocClient;
  readonly table: string;
}

/** The stored-shape version every item is written with. */
export const SCHEMA_VERSION = 1;

/** Name of the GSI that orders chats by update. */
export const GSI1 = 'GSI1';

/**
 * Whether a DynamoDB call failed only because its condition expression was false.
 * @param error - What the send rejected with.
 * @returns True for `ConditionalCheckFailedException`.
 */
const isConditionFailure = (error: unknown): boolean =>
  errorTypeOf(error) === 'ConditionalCheckFailedException';

/**
 * Sends one command; a rejected condition becomes `onCondition`, anything else
 * `UPSTREAM_UNAVAILABLE` naming the operation and the SDK error type.
 * @param run - Performs the send.
 * @param operation - e.g. `dynamo.UpdateItem`, for the error details.
 * @param onCondition - The typed error a failed condition means (defaults to upstream).
 * @returns The output or the typed error.
 */
export const guarded = async <T>(
  run: () => Promise<T>,
  operation: string,
  onCondition?: () => AppError,
): Promise<Result<T, AppError>> => {
  try {
    return ok(await run());
  } catch (error) {
    if (onCondition !== undefined && isConditionFailure(error)) return err(onCondition());
    return err(upstreamError(operation, error));
  }
};

/**
 * Encodes a `LastEvaluatedKey` as an opaque cursor.
 * @param key - The key DynamoDB returned (string attributes only).
 * @returns base64url JSON, or `null` when there is no next page.
 */
export const encodeCursor = (key: Record<string, unknown> | undefined): string | null =>
  key === undefined ? null : Buffer.from(JSON.stringify(key), 'utf8').toString('base64url');

/**
 * Decodes a cursor this adapter issued; it must hold string attributes and the expected partition.
 * @param cursor - From the client, or undefined for the first page.
 * @param partition - Attribute name and value the key must carry (stops cross-owner cursors).
 * @returns The `ExclusiveStartKey`, or `VALIDATION_FAILED`.
 */
export const decodeCursor = (
  cursor: string | undefined,
  partition: readonly [string, string],
): Result<Record<string, string> | undefined, AppError> => {
  if (cursor === undefined) return ok(undefined);
  const bad = err(appError('VALIDATION_FAILED', 'cursor is not valid'));
  if (cursor.length > MAX_CURSOR) return bad;
  try {
    const value: unknown = JSON.parse(Buffer.from(cursor, 'base64url').toString('utf8'));
    if (typeof value !== 'object' || value === null) return bad;
    const entries = Object.entries(value);
    const isStrings = entries.length <= 4 && entries.every(([, v]) => typeof v === 'string');
    const [name, expected] = partition;
    return isStrings && (value as Record<string, string>)[name] === expected
      ? ok(value as Record<string, string>)
      : bad;
  } catch {
    return bad;
  }
};

/**
 * Parses a stored item with its row schema; a malformed item is `INTERNAL` (never a guess).
 * @param schema - Anything with `safeParse` (a Zod row schema).
 * @param item - The raw item.
 * @param what - Names the item type for the error.
 * @returns The parsed row or `INTERNAL`.
 */
export const parseRow = <T>(
  schema: { safeParse(v: unknown): { success: true; data: T } | { success: false } },
  item: unknown,
  what: string,
): Result<T, AppError> => {
  const parsed = schema.safeParse(item);
  return parsed.success
    ? ok(parsed.data)
    : err(appError('INTERNAL', `stored ${what} is malformed`));
};

/**
 * The UpdateItem parts that set each defined field, bump `version` and require the stored version
 * to equal `version` (optimistic lock, stack/data-and-migrations.md).
 * @param fields - Attribute name to new value; `undefined` values are left unchanged.
 * @param version - The version the caller read.
 * @returns UpdateExpression, ConditionExpression and the name/value maps.
 */
export const versionedUpdate = (fields: Readonly<Record<string, unknown>>, version: number) => {
  const sets = ['#v = #v + :one'];
  const names: Record<string, string> = { '#v': 'version' };
  const values: Record<string, unknown> = { ':one': 1, ':expected': version };
  Object.entries(fields).forEach(([field, value], i) => {
    if (value === undefined) return;
    sets.push(`#f${String(i)} = :f${String(i)}`);
    names[`#f${String(i)}`] = field;
    values[`:f${String(i)}`] = value;
  });
  return {
    UpdateExpression: `SET ${sets.join(', ')}`,
    ConditionExpression: '#v = :expected',
    ExpressionAttributeNames: names,
    ExpressionAttributeValues: values,
  };
};

/**
 * Maps every item with `map`, failing on the first malformed one.
 * @param items - Raw items from a query or batch read.
 * @param map - Parses one item.
 * @returns The mapped values in order, or the first error.
 */
export const mapAll = <T>(
  items: readonly unknown[],
  map: (item: unknown) => Result<T, AppError>,
): Result<T[], AppError> => {
  const out: T[] = [];
  for (const item of items) {
    const value = map(item);
    if (!value.ok) return value;
    out.push(value.value);
  }
  return ok(out);
};

/**
 * The items a BatchGetItem returned for one table. Unprocessed keys (throttling) are an
 * `UPSTREAM_UNAVAILABLE`, never a silently shorter result.
 * @param output - The BatchGetItem output.
 * @param table - The table read.
 * @returns The items (any order), or the typed error.
 */
export const batchItems = (
  output: {
    readonly Responses?: Record<string, Record<string, unknown>[]> | undefined;
    readonly UnprocessedKeys?: Record<string, unknown> | undefined;
  },
  table: string,
): Result<Record<string, unknown>[], AppError> =>
  Object.keys(output.UnprocessedKeys ?? {}).length > 0
    ? err(
        appError('UPSTREAM_UNAVAILABLE', 'batch read throttled', {
          operation: 'dynamo.BatchGetItem',
        }),
      )
    : ok(output.Responses?.[table] ?? []);
