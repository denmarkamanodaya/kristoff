// GET /v1/chats use case (docs/contract.md "Routes"): newest `updatedAt` first, pages of 50.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { CHATS_PAGE_SIZE } from '../../domain/limits.js';
import type { ChatSummary, Page, Principal } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import type { Result } from '../../shared/result/result.js';

/**
 * Builds the list-chats use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - The chat store.
 * @returns A function from the principal and opaque cursor to one page of up to 50 chats, or
 *   `VALIDATION_FAILED` for a foreign cursor.
 */
export const listChats =
  (deps: { readonly chats: ChatStore }) =>
  (
    principal: Principal,
    cursor: string | undefined,
  ): Promise<Result<Page<ChatSummary>, AppError>> =>
    deps.chats.listChats(principal.uid, cursor, CHATS_PAGE_SIZE);
