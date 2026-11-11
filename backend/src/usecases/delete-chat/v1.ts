// DELETE /v1/chats/{id} use case (docs/contract.md "Thread deletion"). The chat is read under the
// owner first (ownership check), then every message is deleted, and the chat item LAST, so a
// failure midway leaves the thread visible and the delete can simply be repeated. Documents and
// usage counters are not touched. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { ChatStore } from '../../ports/chat-store.js';
import { forOwnedChat } from '../load-owned-chat/index.js';

/**
 * Builds the delete-chat use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The chat store.
 * @returns A function from the principal and chat id to ok, or `CHAT_NOT_FOUND` (unknown,
 *   another owner's, or already deleted) and store failures (the chat item then still exists).
 */
export const deleteChat = (deps: { readonly chats: ChatStore }) =>
  forOwnedChat(deps.chats, async (_owned, principal, chatId) => {
    const messages = await deps.chats.deleteMessages(chatId);
    if (!messages.ok) return messages;
    return deps.chats.deleteChat(principal.uid, chatId);
  });
