// Loads the principal's chat or answers CHAT_NOT_FOUND: the one ownership read every chat use case
// starts with (get-chat, update-chat, ask-question, delete-chat), and `forOwnedChat`, which wraps a
// `(principal, chatId)` use case in that read. The read is scoped by the owner
// key, so another owner's chat is indistinguishable from a missing one (docs/contract.md "Storage").
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Chat, Principal, Versioned } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import type { Result } from '../../shared/result/result.js';

/**
 * Reads the principal's chat.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param chats - The chat store.
 * @param principal - The verified caller (its `uid` scopes the read).
 * @param chatId - 16-hex chat id.
 * @returns The chat with its version, `CHAT_NOT_FOUND` (missing or another owner's), or a store
 *   failure.
 */
export const loadOwnedChat = (
  chats: ChatStore,
  principal: Principal,
  chatId: string,
): Promise<Result<Versioned<Chat>, AppError>> => chats.getChat(principal.uid, chatId);

/** What a chat use case does once the owner's chat is loaded. */
type OwnedChatAction<T> = (
  owned: Versioned<Chat>,
  principal: Principal,
  chatId: string,
) => Promise<Result<T, AppError>>;

/**
 * Builds a `(principal, chatId)` use case that first loads the owner's chat, then runs `action`.
 * @param chats - The chat store.
 * @param action - What to do with the loaded chat.
 * @returns The use case: `action`'s result, or `CHAT_NOT_FOUND` / a store failure (then `action`
 *   never runs).
 */
export const forOwnedChat =
  <T>(chats: ChatStore, action: OwnedChatAction<T>) =>
  async (principal: Principal, chatId: string): Promise<Result<T, AppError>> => {
    const owned = await loadOwnedChat(chats, principal, chatId);
    return owned.ok ? action(owned.value, principal, chatId) : owned;
  };
