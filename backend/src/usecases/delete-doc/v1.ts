// DELETE /v1/docs/{id} use case (docs/contract.md "v2 File delete"). Order: the doc must exist under
// the owner; then it (and its part ids) is removed from every owner chat's docIds (version-locked,
// `updatedAt` kept so the chat does not jump to the top); then every S3 version of the original and
// its parts is deleted; the doc item goes LAST, so a failure midway can simply be repeated.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { isAttachmentOf } from '../../domain/docs/attachment-id/index.js';
import { docObjectKey, docPartsPrefix } from '../../domain/keys/index.js';
import type { Principal } from '../../domain/model.js';
import type { ChatStore } from '../../ports/chat-store.js';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { ok, type Result } from '../../shared/result/result.js';
import { eachOwnerChat } from '../each-owner-chat/index.js';
import { loadOwnedChat } from '../load-owned-chat/index.js';

/** What delete-doc needs. */
export interface DeleteDocDeps {
  readonly docs: DocStore;
  readonly chats: ChatStore;
  readonly files: FileStore;
}

/** Removes the doc's attachment ids from one chat, at the version just read. */
const unlinkFromChat = async (
  deps: DeleteDocDeps,
  principal: Principal,
  ids: { readonly chatId: string; readonly docId: string },
): Promise<Result<void, AppError>> => {
  const chat = await loadOwnedChat(deps.chats, principal, ids.chatId);
  if (!chat.ok) return chat.error.code === 'CHAT_NOT_FOUND' ? ok(undefined) : chat;
  const { docIds, updatedAt } = chat.value.value;
  const kept = docIds.filter((id) => !isAttachmentOf(id, ids.docId));
  const patch = { docIds: kept, updatedAt };
  const saved = await deps.chats.updateChat(principal.uid, ids.chatId, patch, chat.value.version);
  return saved.ok ? ok(undefined) : saved;
};

const unlinkEverywhere = (
  deps: DeleteDocDeps,
  principal: Principal,
  docId: string,
): Promise<Result<void, AppError>> =>
  eachOwnerChat(deps.chats, principal.uid, {
    select: (chat) => chat.docIds.some((id) => isAttachmentOf(id, docId)),
    visit: (chat) => unlinkFromChat(deps, principal, { chatId: chat.id, docId }),
  });

/**
 * Builds the delete-doc use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Doc, chat and file stores.
 * @returns A function from the principal and doc id to ok, or `DOC_NOT_FOUND`, `VERSION_CONFLICT`
 *   and store failures (the doc item then still exists).
 */
export const deleteDoc =
  (deps: DeleteDocDeps) =>
  async (principal: Principal, docId: string): Promise<Result<void, AppError>> => {
    const found = await deps.docs.get(principal.uid, docId);
    if (!found.ok) return found;
    const unlinked = await unlinkEverywhere(deps, principal, docId);
    if (!unlinked.ok) return unlinked;
    for (const prefix of [
      docObjectKey(principal.uid, docId),
      docPartsPrefix(principal.uid, docId),
    ]) {
      const purged = await deps.files.purgePrefix(prefix);
      if (!purged.ok) return purged;
    }
    return deps.docs.remove(principal.uid, docId);
  };
