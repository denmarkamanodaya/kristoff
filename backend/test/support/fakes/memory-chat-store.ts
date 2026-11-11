// In-memory ChatStore; trusted because test/ports runs the same contract suite as the real adapter.
import type { Chat, ChatSummary, Message, Page, Versioned } from '../../../src/domain/model.js';
import type { ChatPatch, ChatStore } from '../../../src/ports/chat-store.js';
import { appError, type AppError } from '../../../src/shared/errors/app-error.js';
import { err, ok, type Result } from '../../../src/shared/result/result.js';

const summary = ({ id, title, docIds, updatedAt }: Chat): ChatSummary => ({
  id,
  title,
  docIds,
  updatedAt,
});

/** The fake plus its raw maps for assertions. */
export const memoryChatStore = () => {
  const chats = new Map<string, { chat: Chat; version: number }>();
  const messages = new Map<string, Message[]>();
  const k = (uid: string, id: string) => `${uid}/${id}`;
  const store: ChatStore = {
    createChat: (uid, chat) => {
      if (chats.has(k(uid, chat.id))) return Promise.resolve(err(appError('INTERNAL', 'exists')));
      chats.set(k(uid, chat.id), { chat, version: 1 });
      return Promise.resolve(ok(undefined));
    },
    getChat: (uid, id): Promise<Result<Versioned<Chat>, AppError>> => {
      const row = chats.get(k(uid, id));
      return Promise.resolve(
        row ? ok({ value: row.chat, version: row.version }) : err(appError('CHAT_NOT_FOUND', 'no')),
      );
    },
    listChats: (uid, cursor, limit): Promise<Result<Page<ChatSummary>, AppError>> => {
      if (cursor !== undefined && !/^\d+$/.test(cursor))
        return Promise.resolve(err(appError('VALIDATION_FAILED', 'bad cursor')));
      const all = [...chats.entries()]
        .filter(([key]) => key.startsWith(`${uid}/`))
        .map(([, row]) => row.chat)
        .sort((a, b) => `${b.updatedAt}#${b.id}`.localeCompare(`${a.updatedAt}#${a.id}`));
      const start = cursor === undefined ? 0 : Number(cursor);
      const next = start + limit < all.length ? String(start + limit) : null;
      return Promise.resolve(
        ok({ items: all.slice(start, start + limit).map(summary), nextCursor: next }),
      );
    },
    updateChat: (uid, id, patch: ChatPatch, version) => {
      const row = chats.get(k(uid, id));
      if (!row) return Promise.resolve(err(appError('CHAT_NOT_FOUND', 'no')));
      if (row.version !== version)
        return Promise.resolve(err(appError('VERSION_CONFLICT', 'stale')));
      const chat = { ...row.chat, ...patch };
      chats.set(k(uid, id), { chat, version: version + 1 });
      return Promise.resolve(ok(chat));
    },
    putMessage: (chatId, message) => {
      const list = messages.get(chatId) ?? [];
      if (list.some((m) => m.id === message.id))
        return Promise.resolve(err(appError('INTERNAL', 'exists')));
      messages.set(
        chatId,
        [...list, message].sort((a, b) => a.id.localeCompare(b.id)),
      );
      return Promise.resolve(ok(undefined));
    },
    deleteMessages: (chatId) => {
      const count = messages.get(chatId)?.length ?? 0;
      messages.delete(chatId);
      return Promise.resolve(ok(count));
    },
    deleteChat: (uid, chatId) => {
      chats.delete(k(uid, chatId));
      return Promise.resolve(ok(undefined));
    },
    recentMessages: (chatId, limit) =>
      Promise.resolve(ok(limit <= 0 ? [] : (messages.get(chatId) ?? []).slice(-limit))),
  };
  return { store, chats, messages };
};
