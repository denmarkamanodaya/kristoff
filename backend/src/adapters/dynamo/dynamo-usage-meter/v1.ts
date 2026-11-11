// DynamoUsageMeter: the UsageMeter port (docs/contract.md "Storage", Usage day/total). The daily
// cap is one conditional `ADD questions :1` with `questions < :cap`, before the model call;
// tokens are added to the day and the total after it. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { BatchGetCommand, UpdateCommand } from '@aws-sdk/lib-dynamodb';
import { usageDayKey, usageTotalKey, type ItemKey } from '../../../domain/keys/index.js';
import type { Usage } from '../../../domain/model.js';
import type { UsageMeter } from '../../../ports/usage-meter.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { ok, type Result } from '../../../shared/result/result.js';
import {
  batchItems,
  guarded,
  parseRow,
  SCHEMA_VERSION,
  type DynamoTarget,
} from '../_shared/dynamo-call.js';
import { UsageRow } from '../_shared/rows.js';

const toUsage = (item: unknown): Result<Usage, AppError> => {
  const row = parseRow(UsageRow, item ?? { SK: '' }, 'usage');
  if (!row.ok) return row;
  const { input = 0, output = 0, cacheRead = 0, cacheWrite = 0 } = row.value;
  return ok({ input, output, cacheRead, cacheWrite });
};

const addTo = (target: DynamoTarget, key: ItemKey, usage: Usage) =>
  guarded(
    () =>
      target.client.send(
        new UpdateCommand({
          TableName: target.table,
          Key: key,
          UpdateExpression:
            'ADD #in :in, #out :out, cacheRead :cr, cacheWrite :cw, #v :one SET schemaVersion = :sv',
          ExpressionAttributeNames: { '#in': 'input', '#out': 'output', '#v': 'version' },
          ExpressionAttributeValues: {
            ':in': usage.input,
            ':out': usage.output,
            ':cr': usage.cacheRead,
            ':cw': usage.cacheWrite,
            ':one': 1,
            ':sv': SCHEMA_VERSION,
          },
        }),
      ),
    'dynamo.UpdateItem',
  );

const admitQuestion =
  (target: DynamoTarget): UsageMeter['admitQuestion'] =>
  async (uid, day, cap) => {
    const admitted = await guarded(
      () =>
        target.client.send(
          new UpdateCommand({
            TableName: target.table,
            Key: usageDayKey(uid, day),
            UpdateExpression: 'ADD questions :one, #v :one SET schemaVersion = :sv',
            ConditionExpression: 'attribute_not_exists(questions) OR questions < :cap',
            ExpressionAttributeNames: { '#v': 'version' },
            ExpressionAttributeValues: { ':one': 1, ':cap': cap, ':sv': SCHEMA_VERSION },
          }),
        ),
      'dynamo.UpdateItem',
      () => appError('DAILY_CAP_REACHED', 'daily question limit reached'),
    );
    return admitted.ok ? ok(undefined) : admitted;
  };

const read =
  (target: DynamoTarget): UsageMeter['read'] =>
  async (uid, day) => {
    const keys = [usageDayKey(uid, day), usageTotalKey(uid)];
    const got = await guarded(
      () =>
        target.client.send(
          new BatchGetCommand({ RequestItems: { [target.table]: { Keys: keys } } }),
        ),
      'dynamo.BatchGetItem',
    );
    if (!got.ok) return got;
    const items = batchItems(got.value, target.table);
    if (!items.ok) return items;
    const find = (sk: string): unknown => items.value.find((i) => i.SK === sk);
    const today = toUsage(find(keys[0]?.SK ?? ''));
    const total = toUsage(find(keys[1]?.SK ?? ''));
    if (!today.ok) return today;
    return total.ok ? ok({ today: today.value, total: total.value }) : total;
  };

/**
 * Builds the DynamoDB usage meter.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Document client and table name.
 * @returns The UsageMeter.
 */
export const dynamoUsageMeter = (target: DynamoTarget): UsageMeter => ({
  admitQuestion: admitQuestion(target),
  addTokens: async (uid, day, usage) => {
    const [daily, total] = await Promise.all([
      addTo(target, usageDayKey(uid, day), usage),
      addTo(target, usageTotalKey(uid), usage),
    ]);
    if (!daily.ok) return daily;
    return total.ok ? ok(undefined) : total;
  },
  read: read(target),
});
