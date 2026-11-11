// DynamoChatStore: the ChatStore port (docs/contract.md "Storage", Chat and Message items). Chats
// live under the owner with GSI1 ordering them by update; messages under `CHAT#<id>` sorted by
// ULID. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import {
  chatGsiKey,
  chatKey,
  chatPk,
  chatsGsiPk,
  CHAT_SK_PREFIX,
  messageKey,
  MESSAGE_SK_PREFIX,
  type ItemKey,
} from '../../../domain/keys/index.js';
import type { Chat, ChatSummary, Message, Versioned } from '../../../domain/model.js';
import type { ChatStore } from '../../../ports/chat-store.js';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';
import {
  decodeCursor,
  encodeCursor,
  GSI1,
  mapAll,
  parseRow,
  SCHEMA_VERSION,
  type DynamoTarget,
} from '../_shared/dynamo-call.js';
import { deleteItem, getItem, putNew, queryPage, updateVersioned } from '../_shared/item-ops.js';
import { mapLimit } from '../../../shared/map-limit/map-limit.js';
import { ChatRow, MessageRow } from '../_shared/rows.js';

/** Largest serialised message item written, in bytes (DynamoDB caps an item at 400 KB). */
const MAX_ITEM_BYTES = 390_000;

const toChat = (item: unknown): Result<Versioned<Chat>, AppError> => {
  const row = parseRow(ChatRow, item, 'chat');
  if (!row.ok) return row;
  const { SK, title, docIds, createdAt, updatedAt, version } = row.value;
  const id = SK.slice(CHAT_SK_PREFIX.length);
  return ok({ value: { id, title, docIds, createdAt, updatedAt }, version });
};

const toSummary = (item: unknown): Result<ChatSummary, AppError> => {
  const chat = toChat(item);
  if (!chat.ok) return chat;
  const { id, title, docIds, updatedAt } = chat.value.value;
  return ok({ id, title, docIds, updatedAt });
};

const toMessage = (item: unknown): Result<Message, AppError> => {
  const row = parseRow(MessageRow, item, 'message');
  if (!row.ok) return row;
  const { id, role, text, citations, usage, model, stopReason, at } = row.value;
  return ok({ id, role, text, citations, usage, model, stopReason, at });
};

const listChats =
  (target: DynamoTarget): ChatStore['listChats'] =>
  async (uid, cursor, limit) => {
    const start = decodeCursor(cursor, ['GSI1PK', chatsGsiPk(uid)]);
    if (!start.ok) return start;
    const page = await queryPage(target, {
      IndexName: GSI1,
      KeyConditionExpression: 'GSI1PK = :pk',
      ExpressionAttributeValues: { ':pk': chatsGsiPk(uid) },
      ScanIndexForward: false,
      Limit: limit,
      ExclusiveStartKey: start.value,
    });
    if (!page.ok) return page;
    const items = mapAll(page.value.items, toSummary);
    return items.ok
      ? ok({ items: items.value, nextCursor: encodeCursor(page.value.lastKey) })
      : items;
  };

const recentMessages =
  (target: DynamoTarget): ChatStore['recentMessages'] =>
  async (chatId, limit) => {
    if (limit <= 0) return ok([]);
    const page = await queryPage(target, {
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
      ExpressionAttributeValues: { ':pk': chatPk(chatId), ':sk': MESSAGE_SK_PREFIX },
      ScanIndexForward: false,
      Limit: limit,
      ConsistentRead: true,
    });
    if (!page.ok) return page;
    const messages = mapAll(page.value.items, toMessage);
    return messages.ok ? ok(messages.value.reverse()) : messages;
  };

const createChat =
  (target: DynamoTarget): ChatStore['createChat'] =>
  (uid, chat) =>
    putNew(
      target,
      {
        ...chatKey(uid, chat.id),
        ...chatGsiKey(uid, chat.updatedAt, chat.id),
        title: chat.title,
        docIds: chat.docIds,
        createdAt: chat.createdAt,
        updatedAt: chat.updatedAt,
        schemaVersion: SCHEMA_VERSION,
        version: 1,
      },
      () => appError('INTERNAL', 'chat id already exists'),
    );

const getChat =
  (target: DynamoTarget): ChatStore['getChat'] =>
  async (uid, chatId) => {
    const got = await getItem(target, chatKey(uid, chatId));
    if (!got.ok) return got;
    return got.value ? toChat(got.value) : err(appError('CHAT_NOT_FOUND', 'no such chat'));
  };

/** The primary key of a projected item; `INTERNAL` when it is not two strings. */
const toKey = (item: unknown): Result<ItemKey, AppError> => {
  const { PK, SK } = (item ?? {}) as Record<string, unknown>;
  return typeof PK === 'string' && typeof SK === 'string'
    ? ok({ PK, SK })
    : err(appError('INTERNAL', 'stored message key is malformed'));
};

/** Message keys read per page while deleting (bounded query). */
const DELETE_PAGE = 100;
/** Most DeleteItem calls in flight at once (docs/contract.md "Thread deletion"). */
const DELETE_CONCURRENCY = 8;

/** Deletes the keys, at most 8 at a time; the first failure is returned (the rest still ran). */
const deleteKeys = async (
  target: DynamoTarget,
  keys: readonly ItemKey[],
): Promise<Result<void, AppError>> => {
  const results = await mapLimit(keys, DELETE_CONCURRENCY, (key) => deleteItem(target, key));
  return results.find((r) => !r.ok) ?? ok(undefined);
};

const deleteMessages =
  (target: DynamoTarget): ChatStore['deleteMessages'] =>
  async (chatId) => {
    let deleted = 0;
    let start: Record<string, unknown> | undefined;
    do {
      const page = await queryPage(target, {
        KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk)',
        ExpressionAttributeValues: { ':pk': chatPk(chatId), ':sk': MESSAGE_SK_PREFIX },
        ProjectionExpression: 'PK, SK',
        Limit: DELETE_PAGE,
        ConsistentRead: true,
        ExclusiveStartKey: start,
      });
      if (!page.ok) return page;
      const keys = mapAll(page.value.items, toKey);
      if (!keys.ok) return keys;
      const done = await deleteKeys(target, keys.value);
      if (!done.ok) return done;
      deleted += keys.value.length;
      start = page.value.lastKey;
    } while (start !== undefined);
    return ok(deleted);
  };

const deleteChat =
  (target: DynamoTarget): ChatStore['deleteChat'] =>
  (uid, chatId) =>
    deleteItem(target, chatKey(uid, chatId));

const putMessage =
  (target: DynamoTarget): ChatStore['putMessage'] =>
  async (chatId, message) => {
    const item = {
      ...messageKey(chatId, message.id),
      ...message,
      schemaVersion: SCHEMA_VERSION,
      version: 1,
    };
    if (Buffer.byteLength(JSON.stringify(item), 'utf8') > MAX_ITEM_BYTES) {
      return err(
        appError('INTERNAL', 'message too large to store', { operation: 'dynamo.PutItem' }),
      );
    }
    return putNew(target, item, () => appError('INTERNAL', 'message id already exists'));
  };

/**
 * Builds the DynamoDB chat store.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Document client and table name.
 * @returns The ChatStore. `putMessage` refuses (`INTERNAL`) an item over 390 KB rather than let
 *   DynamoDB fail it opaquely.
 */
export const dynamoChatStore = (target: DynamoTarget): ChatStore => ({
  createChat: createChat(target),
  getChat: getChat(target),
  listChats: listChats(target),
  updateChat: async (uid, chatId, patch, version) => {
    const { GSI1SK } = chatGsiKey(uid, patch.updatedAt, chatId);
    const fields = { title: patch.title, docIds: patch.docIds, updatedAt: patch.updatedAt, GSI1SK };
    const onConflict = () => appError('VERSION_CONFLICT', 'chat changed since it was read');
    const updated = await updateVersioned(target, chatKey(uid, chatId), fields, {
      version,
      onConflict,
    });
    if (!updated.ok) return updated;
    const chat = toChat(updated.value);
    return chat.ok ? ok(chat.value.value) : chat;
  },
  putMessage: putMessage(target),
  recentMessages: recentMessages(target),
  deleteMessages: deleteMessages(target),
  deleteChat: deleteChat(target),
});
