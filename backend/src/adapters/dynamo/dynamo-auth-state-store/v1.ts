// DynamoAuthStateStore: the AuthStateStore port (docs/contract.md "Auth flow" step 5, TOTP state
// item). The replay guard is one conditional update `lastStep < :step`; failures are
// version-locked writes. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { PutCommand, UpdateCommand } from '@aws-sdk/lib-dynamodb';
import { INITIAL_TOTP_STATE } from '../../../domain/auth/totp-state.js';
import { totpKey } from '../../../domain/keys/index.js';
import type { AuthStateStore } from '../../../ports/auth-state-store.js';
import { appError } from '../../../shared/errors/app-error.js';
import { ok } from '../../../shared/result/result.js';
import { guarded, parseRow, SCHEMA_VERSION, type DynamoTarget } from '../_shared/dynamo-call.js';
import { getItem } from '../_shared/item-ops.js';
import { TotpRow } from '../_shared/rows.js';

const get =
  (target: DynamoTarget): AuthStateStore['get'] =>
  async (uid) => {
    const got = await getItem(target, totpKey(uid));
    if (!got.ok) return got;
    if (got.value === undefined) return ok({ value: INITIAL_TOTP_STATE, version: 0 });
    const row = parseRow(TotpRow, got.value, 'TOTP state');
    if (!row.ok) return row;
    const { lastStep, failCount, lockedUntil, version } = row.value;
    return ok({ value: { lastStep, failCount, lockedUntil }, version });
  };

const acceptStep =
  (target: DynamoTarget): AuthStateStore['acceptStep'] =>
  async (uid, step) => {
    const updated = await guarded(
      () =>
        target.client.send(
          new UpdateCommand({
            TableName: target.table,
            Key: totpKey(uid),
            UpdateExpression:
              'SET lastStep = :step, failCount = :zero, lockedUntil = :zero, schemaVersion = :sv, #v = if_not_exists(#v, :zero) + :one',
            ConditionExpression: 'attribute_not_exists(lastStep) OR lastStep < :step',
            ExpressionAttributeNames: { '#v': 'version' },
            ExpressionAttributeValues: {
              ':step': step,
              ':zero': 0,
              ':one': 1,
              ':sv': SCHEMA_VERSION,
            },
          }),
        ),
      'dynamo.UpdateItem',
      () => appError('OTP_REPLAYED', 'this code was already used; wait for the next one'),
    );
    return updated.ok ? ok(undefined) : updated;
  };

const recordFailure =
  (target: DynamoTarget): AuthStateStore['recordFailure'] =>
  async (uid, next, version) => {
    const item = { ...totpKey(uid), ...next, schemaVersion: SCHEMA_VERSION, version: version + 1 };
    const condition =
      version === 0
        ? { ConditionExpression: 'attribute_not_exists(PK)' }
        : {
            ConditionExpression: '#v = :expected',
            ExpressionAttributeNames: { '#v': 'version' },
            ExpressionAttributeValues: { ':expected': version },
          };
    const put = await guarded(
      () =>
        target.client.send(new PutCommand({ TableName: target.table, Item: item, ...condition })),
      'dynamo.PutItem',
      () => appError('VERSION_CONFLICT', 'TOTP state changed since it was read'),
    );
    return put.ok ? ok(undefined) : put;
  };

/**
 * Builds the DynamoDB TOTP state store.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Document client and table name.
 * @returns The AuthStateStore.
 */
export const dynamoAuthStateStore = (target: DynamoTarget): AuthStateStore => ({
  get: get(target),
  acceptStep: acceptStep(target),
  recordFailure: recordFailure(target),
});
