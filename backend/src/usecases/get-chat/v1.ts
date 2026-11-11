// GET /v1/chats/{id} use case (docs/contract.md "Routes"): the chat plus its newest 200 messages,
// oldest first. The chat is read under the owner first, so another owner's messages are never
// read. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { CHAT_MESSAGES_MAX } from '../../domain/limits.js';
import type { Chat, Message, Principal } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

/** A chat with its messages. */
export interface ChatWithMessages {
  readonly chat: Chat;
  readonly messages: readonly Message[];
}

/**
 * Builds the get-chat use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The chat store.
 * @returns A function from the principal and chat id to the chat and messages, or
 *   `CHAT_NOT_FOUND`.
 */
export const getChat =
  (deps: { readonly chats: ChatStore }) =>
  async (principal: Principal, chatId: string): Promise<Result<ChatWithMessages, AppError>> => {
    const found = await deps.chats.getChat(principal.uid, chatId);
    if (!found.ok) return found;
    const messages = await deps.chats.recentMessages(chatId, CHAT_MESSAGES_MAX);
    return messages.ok ? ok({ chat: found.value.value, messages: messages.value }) : messages;
  };
