// POST /v1/chats use case (docs/contract.md "Routes"): a new chat titled "New chat" with optional
// ready docs attached. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { NEW_CHAT_TITLE } from '../../domain/limits.js';
import type { Chat, Principal } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { Clock } from '../../ports/clock.js';
import type { ResolveDocs } from '../resolve-attached-docs/index.js';
import type { IdGenerator } from '../../ports/id-generator.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';

/** What create-chat needs. */
export interface CreateChatDeps {
  readonly chats: ChatStore;
  readonly ids: IdGenerator;
  readonly clock: Clock;
  /** The resolve-attached-docs use case, bound to the configured cap. */
  readonly resolveDocs: ResolveDocs;
}

/**
 * Builds the create-chat use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Chat store, ids, clock and the attached-docs check.
 * @returns A function from the principal and doc ids to the new chat, or `VALIDATION_FAILED` /
 *   `DOC_NOT_FOUND` from the docs check and store failures.
 */
export const createChat =
  (deps: CreateChatDeps) =>
  async (principal: Principal, docIds: readonly string[]): Promise<Result<Chat, AppError>> => {
    const resolved = await deps.resolveDocs(principal, docIds);
    if (!resolved.ok) return resolved;
    const now = new Date(deps.clock.now()).toISOString();
    const chat: Chat = {
      id: deps.ids.newId(),
      title: NEW_CHAT_TITLE,
      docIds: resolved.value,
      createdAt: now,
      updatedAt: now,
    };
    const created = await deps.chats.createChat(principal.uid, chat);
    return created.ok ? ok(chat) : created;
  };
