// PATCH /v1/chats/{id} use case (docs/contract.md "Routes"): renames and/or re-attaches docs.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Chat, Principal } from '../../domain/model.js';
import type { ChatPatch, ChatStore } from '../../ports/chat-store.js';
import type { Clock } from '../../ports/clock.js';
import type { ResolveDocs } from '../resolve-attached-docs/index.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, type Result } from '../../shared/result/result.js';

/** What update-chat needs. */
export interface UpdateChatDeps {
  readonly chats: ChatStore;
  readonly clock: Clock;
  /** The resolve-attached-docs use case, bound to the configured cap. */
  readonly resolveDocs: ResolveDocs;
}

/** The fields to change; at least one is present (the request schema enforces it). */
export interface ChatChanges {
  readonly title?: string;
  readonly docIds?: readonly string[];
}

const resolvePatch = async (
  deps: UpdateChatDeps,
  principal: Principal,
  changes: ChatChanges,
): Promise<Result<ChatPatch, AppError>> => {
  const updatedAt = new Date(deps.clock.now()).toISOString();
  const title = changes.title?.trim();
  if (title === '') return err(appError('VALIDATION_FAILED', 'title is blank'));
  const base = { updatedAt, ...(title === undefined ? {} : { title }) };
  if (changes.docIds === undefined) return { ok: true, value: base };
  const docIds = await deps.resolveDocs(principal, changes.docIds);
  return docIds.ok ? { ok: true, value: { ...base, docIds: docIds.value } } : docIds;
};

/**
 * Builds the update-chat use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Chat store, clock and the attached-docs check.
 * @returns A function from the principal, chat id and changes to the updated chat, or
 *   `CHAT_NOT_FOUND`, `VALIDATION_FAILED`, `DOC_NOT_FOUND` and `VERSION_CONFLICT`.
 */
export const updateChat =
  (deps: UpdateChatDeps) =>
  async (
    principal: Principal,
    chatId: string,
    changes: ChatChanges,
  ): Promise<Result<Chat, AppError>> => {
    const found = await deps.chats.getChat(principal.uid, chatId);
    if (!found.ok) return found;
    const patch = await resolvePatch(deps, principal, changes);
    if (!patch.ok) return patch;
    return deps.chats.updateChat(principal.uid, chatId, patch.value, found.value.version);
  };
