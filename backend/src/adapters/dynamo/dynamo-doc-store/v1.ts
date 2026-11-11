// DynamoDocStore: the DocStore port on the single table (docs/contract.md "Storage", Doc item).
// Conditional create, version-locked update, bounded paginated query, opaque cursor. Lists newest
// first: ids are time-ordered (IdGenerator), so `DOC#<id>` descending is createdAt descending.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { BatchGetCommand } from '@aws-sdk/lib-dynamodb';
import { docKey, DOC_SK_PREFIX, ownerPk } from '../../../domain/keys/index.js';
import type { Doc, Versioned } from '../../../domain/model.js';
import type { DocStore } from '../../../ports/doc-store.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';
import {
  batchItems,
  decodeCursor,
  encodeCursor,
  guarded,
  mapAll,
  parseRow,
  SCHEMA_VERSION,
  type DynamoTarget,
} from '../_shared/dynamo-call.js';
import { getItem, putNew, queryPage, updateVersioned } from '../_shared/item-ops.js';
import { DocRow } from '../_shared/rows.js';

const toDoc = (item: unknown): Result<Versioned<Doc>, AppError> => {
  const row = parseRow(DocRow, item, 'doc');
  if (!row.ok) return row;
  const { SK, title, size, status, createdAt, version } = row.value;
  const id = SK.slice(DOC_SK_PREFIX.length);
  return ok({ value: { id, title, size, status, createdAt }, version });
};

const toDocs = (items: readonly unknown[]): Result<Doc[], AppError> =>
  mapAll(items, (item) => {
    const doc = toDoc(item);
    return doc.ok ? ok(doc.value.value) : doc;
  });

const getMany =
  ({ client, table }: DynamoTarget): DocStore['getMany'] =>
  async (uid, docIds) => {
    if (docIds.length === 0) return ok([]);
    const Keys = docIds.map((id) => docKey(uid, id));
    const got = await guarded(
      () =>
        client.send(
          new BatchGetCommand({ RequestItems: { [table]: { Keys, ConsistentRead: true } } }),
        ),
      'dynamo.BatchGetItem',
    );
    if (!got.ok) return got;
    const items = batchItems(got.value, table);
    return items.ok ? toDocs(items.value) : items;
  };

const list =
  (target: DynamoTarget): DocStore['list'] =>
  async (uid, cursor, limit) => {
    const start = decodeCursor(cursor, ['PK', ownerPk(uid)]);
    if (!start.ok) return start;
    const page = await queryPage(target, {
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': ownerPk(uid), ':sk': DOC_SK_PREFIX },
      ScanIndexForward: false,
      Limit: limit,
      ExclusiveStartKey: start.value,
    });
    if (!page.ok) return page;
    const docs = toDocs(page.value.items);
    return docs.ok ? ok({ items: docs.value, nextCursor: encodeCursor(page.value.lastKey) }) : docs;
  };

/**
 * Builds the DynamoDB doc store.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Document client and table name.
 * @returns The DocStore.
 */
export const dynamoDocStore = (target: DynamoTarget): DocStore => ({
  create: (uid, doc, objectKey) =>
    putNew(
      target,
      {
        ...docKey(uid, doc.id),
        title: doc.title,
        size: doc.size,
        status: doc.status,
        createdAt: doc.createdAt,
        s3Key: objectKey,
        schemaVersion: SCHEMA_VERSION,
        version: 1,
      },
      () => appError('INTERNAL', 'doc id already exists'),
    ),
  get: async (uid, docId) => {
    const got = await getItem(target, docKey(uid, docId));
    if (!got.ok) return got;
    return got.value ? toDoc(got.value) : err(appError('DOC_NOT_FOUND', 'no such doc'));
  },
  getMany: getMany(target),
  list: list(target),
  update: async (uid, docId, patch, version) => {
    const fields = { title: patch.title, status: patch.status };
    const onConflict = () => appError('VERSION_CONFLICT', 'doc changed since it was read');
    const updated = await updateVersioned(target, docKey(uid, docId), fields, {
      version,
      onConflict,
    });
    if (!updated.ok) return updated;
    const doc = toDoc(updated.value);
    return doc.ok ? ok(doc.value.value) : doc;
  },
});
