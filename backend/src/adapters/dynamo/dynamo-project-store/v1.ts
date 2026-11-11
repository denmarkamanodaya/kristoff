// DynamoProjectStore: the ProjectStore port (docs/contract.md "v2 Projects"). The cap is one
// transaction: the owner's counter item `COUNT#PROJECTS` is incremented only while `n < :cap`,
// together with a create-if-absent of the project; delete removes the project and decrements.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { TransactWriteCommand } from '@aws-sdk/lib-dynamodb';
import {
  ownerPk,
  projectCountKey,
  projectKey,
  PROJECT_SK_PREFIX,
} from '../../../domain/keys/index.js';
import { MAX_PROJECTS } from '../../../domain/limits.js';
import type { Project, Versioned } from '../../../domain/model.js';
import type { ProjectStore } from '../../../ports/project-store.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';
import { mapAll, parseRow, SCHEMA_VERSION, type DynamoTarget } from '../_shared/dynamo-call.js';
import { getItem, queryPage, updateVersioned } from '../_shared/item-ops.js';
import { ProjectRow } from '../_shared/rows.js';
import { upstreamError } from '../../../shared/errors/vendor-error.js';

const toProject = (item: unknown): Result<Versioned<Project>, AppError> => {
  const row = parseRow(ProjectRow, item, 'project');
  if (!row.ok) return row;
  const { SK, name, createdAt, updatedAt, version } = row.value;
  return ok({
    value: { id: SK.slice(PROJECT_SK_PREFIX.length), name, createdAt, updatedAt },
    version,
  });
};

type TransactItems = NonNullable<
  ConstructorParameters<typeof TransactWriteCommand>[0]['TransactItems']
>;

/**
 * Sends one transaction. A cancellation whose FIRST item failed its condition becomes
 * `onFirstFailed()`; any other failure is `UPSTREAM_UNAVAILABLE`.
 */
const transact = async (
  target: DynamoTarget,
  items: TransactItems,
  onFirstFailed: () => AppError,
): Promise<Result<void, AppError>> => {
  try {
    await target.client.send(new TransactWriteCommand({ TransactItems: items }));
    return ok(undefined);
  } catch (error) {
    const reasons = (error as { CancellationReasons?: { Code?: string }[] }).CancellationReasons;
    const isFirst =
      error instanceof Error &&
      error.name === 'TransactionCanceledException' &&
      reasons?.[0]?.Code === 'ConditionalCheckFailed';
    return err(isFirst ? onFirstFailed() : upstreamError('dynamo.TransactWriteItems', error));
  }
};

const create =
  (target: DynamoTarget): ProjectStore['create'] =>
  (uid, project, cap) => {
    return transact(
      target,
      [
        {
          Update: {
            TableName: target.table,
            Key: projectCountKey(uid),
            UpdateExpression: 'ADD n :one SET schemaVersion = :sv',
            ConditionExpression: 'attribute_not_exists(n) OR n < :cap',
            ExpressionAttributeValues: { ':one': 1, ':cap': cap, ':sv': SCHEMA_VERSION },
          },
        },
        {
          Put: {
            TableName: target.table,
            Item: {
              ...projectKey(uid, project.id),
              name: project.name,
              createdAt: project.createdAt,
              updatedAt: project.updatedAt,
              schemaVersion: SCHEMA_VERSION,
              version: 1,
            },
            ConditionExpression: 'attribute_not_exists(PK)',
          },
        },
      ],
      () => appError('PROJECT_LIMIT_REACHED', `at most ${String(cap)} projects`),
    );
  };

const remove =
  (target: DynamoTarget): ProjectStore['delete'] =>
  (uid, projectId) => {
    return transact(
      target,
      [
        {
          Delete: {
            TableName: target.table,
            Key: projectKey(uid, projectId),
            ConditionExpression: 'attribute_exists(PK)',
          },
        },
        {
          Update: {
            TableName: target.table,
            Key: projectCountKey(uid),
            UpdateExpression: 'ADD n :minus',
            ConditionExpression: 'n > :zero',
            ExpressionAttributeValues: { ':minus': -1, ':zero': 0 },
          },
        },
      ],
      () => appError('PROJECT_NOT_FOUND', 'no such project'),
    );
  };

/**
 * Builds the DynamoDB project store.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Document client and table name.
 * @returns The ProjectStore.
 */
export const dynamoProjectStore = (target: DynamoTarget): ProjectStore => ({
  create: create(target),
  get: async (uid, projectId) => {
    const got = await getItem(target, projectKey(uid, projectId));
    if (!got.ok) return got;
    return got.value ? toProject(got.value) : err(appError('PROJECT_NOT_FOUND', 'no such project'));
  },
  list: async (uid) => {
    const page = await queryPage(target, {
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': ownerPk(uid), ':sk': PROJECT_SK_PREFIX },
      Limit: MAX_PROJECTS,
    });
    if (!page.ok) return page;
    return mapAll(page.value.items, (item) => {
      const project = toProject(item);
      return project.ok ? ok(project.value.value) : project;
    });
  },
  rename: async (uid, projectId, change) => {
    const onConflict = () => appError('VERSION_CONFLICT', 'project changed since it was read');
    const fields = { name: change.name, updatedAt: change.updatedAt };
    const updated = await updateVersioned(target, projectKey(uid, projectId), fields, {
      version: change.version,
      onConflict,
    });
    if (!updated.ok) return updated;
    const project = toProject(updated.value);
    return project.ok ? ok(project.value.value) : project;
  },
  delete: remove(target),
});
