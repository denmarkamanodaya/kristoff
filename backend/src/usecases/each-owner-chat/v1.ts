// Visits every chat of the owner, page by page from GSI1 (newest first): the one walk that
// delete-project (ungroup) and delete-doc (unlink) both need. Stops at the first failure.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { CHATS_PAGE_SIZE } from '../../domain/limits.js';
import type { ChatSummary } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

/**
 * Calls `visit` for each of the owner's chats that `select` keeps, one at a time.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param chats - The chat store.
 * @param uid - Owner id from the principal.
 * @param step - `select` picks the chats to visit; `visit` changes one.
 * @returns Ok when every visit succeeded, else the first failure (later chats are not visited).
 */
export const eachOwnerChat = async (
  chats: ChatStore,
  uid: string,
  step: {
    readonly select: (chat: ChatSummary) => boolean;
    readonly visit: (chat: ChatSummary) => Promise<Result<void, AppError>>;
  },
): Promise<Result<void, AppError>> => {
  let cursor: string | undefined;
  do {
    const page = await chats.listChats(uid, cursor, CHATS_PAGE_SIZE);
    if (!page.ok) return page;
    for (const chat of page.value.items.filter(step.select)) {
      const done = await step.visit(chat);
      if (!done.ok) return done;
    }
    cursor = page.value.nextCursor ?? undefined;
  } while (cursor !== undefined);
  return ok(undefined);
};
