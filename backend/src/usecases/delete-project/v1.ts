// DELETE /v1/projects/{id} use case (docs/contract.md "v2 Projects"): every chat of the project is
// ungrouped first (the owner's chats are paged from GSI1, filtered in code, each cleared with one
// conditional update), then the project item is deleted. A failure midway leaves the project in
// place, so the delete can be repeated. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Principal } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { ProjectStore } from '../../ports/project-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import type { Result } from '../../shared/result/result.js';
import { eachOwnerChat } from '../each-owner-chat/index.js';

/** What delete-project needs. */
export interface DeleteProjectDeps {
  readonly projects: ProjectStore;
  readonly chats: ChatStore;
}

const ungroupChats = (
  deps: DeleteProjectDeps,
  uid: string,
  projectId: string,
): Promise<Result<void, AppError>> =>
  eachOwnerChat(deps.chats, uid, {
    select: (chat) => chat.projectId === projectId,
    visit: (chat) => deps.chats.clearProject(uid, chat.id, projectId),
  });

/**
 * Builds the delete-project use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Project and chat stores.
 * @returns A function from the principal and project id to ok, or `PROJECT_NOT_FOUND` and store
 *   failures (the project then still exists).
 */
export const deleteProject =
  (deps: DeleteProjectDeps) =>
  async (principal: Principal, projectId: string): Promise<Result<void, AppError>> => {
    const found = await deps.projects.get(principal.uid, projectId);
    if (!found.ok) return found;
    const ungrouped = await ungroupChats(deps, principal.uid, projectId);
    if (!ungrouped.ok) return ungrouped;
    return deps.projects.delete(principal.uid, projectId);
  };
