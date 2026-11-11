// ChatStore port: chats and their messages. Contract: docs/contract.md "Storage" (Chat, Message).
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { Chat, ChatSummary, Message, Page, Versioned } from '../domain/model.js';

/** Fields a chat update may change; `updatedAt` always changes. */
export interface ChatPatch {
  readonly title?: string;
  readonly docIds?: readonly string[];
  readonly updatedAt: string;
}

/** Stores an owner's chats and every message. */
export interface ChatStore {
  /** Creates a chat; fails with `INTERNAL` when the id already exists. */
  createChat(uid: string, chat: Chat): Promise<Result<void, AppError>>;
  /** Reads one chat; `CHAT_NOT_FOUND` when absent. */
  getChat(uid: string, chatId: string): Promise<Result<Versioned<Chat>, AppError>>;
  /** Lists chats newest `updatedAt` first; `VALIDATION_FAILED` for a foreign cursor. */
  listChats(
    uid: string,
    cursor: string | undefined,
    limit: number,
  ): Promise<Result<Page<ChatSummary>, AppError>>;
  /** Applies a patch if the stored version still equals `version`; else `VERSION_CONFLICT`. */
  updateChat(
    uid: string,
    chatId: string,
    patch: ChatPatch,
    version: number,
  ): Promise<Result<Chat, AppError>>;
  /** Appends a message; never overwrites an existing id. */
  putMessage(chatId: string, message: Message): Promise<Result<void, AppError>>;
  /** The newest `limit` messages of a chat, returned oldest first. */
  recentMessages(chatId: string, limit: number): Promise<Result<Message[], AppError>>;
  /**
   * Deletes every message of a chat (all pages), one item delete at a time with bounded
   * concurrency. Repeatable: a second call deletes nothing. `UPSTREAM_UNAVAILABLE` when a delete
   * fails (the chat item is untouched, so the caller can repeat).
   * @returns How many messages were deleted.
   */
  deleteMessages(chatId: string): Promise<Result<number, AppError>>;
  /** Deletes the chat item under its owner; deleting an absent chat is not an error. */
  deleteChat(uid: string, chatId: string): Promise<Result<void, AppError>>;
}
