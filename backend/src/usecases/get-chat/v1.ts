// GET /v1/chats/{id} use case (docs/contract.md "Routes"): the chat plus its newest 200 messages,
// oldest first. The chat is read under the owner first, so another owner's messages are never
// read. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { CHAT_MESSAGES_MAX } from '../../domain/limits.js';
import type { Chat, Message } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import { ok } from '../../shared/result/result.js';
import { forOwnedChat } from '../load-owned-chat/index.js';

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
export const getChat = (deps: { readonly chats: ChatStore }) =>
  forOwnedChat<ChatWithMessages>(deps.chats, async (owned, _principal, chatId) => {
    const messages = await deps.chats.recentMessages(chatId, CHAT_MESSAGES_MAX);
    return messages.ok ? ok({ chat: owned.value, messages: messages.value }) : messages;
  });
