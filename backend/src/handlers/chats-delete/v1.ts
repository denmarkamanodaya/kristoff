// DELETE /v1/chats/{id} handler (docs/contract.md "Thread deletion"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { chatsDeleteDeps } from '../../composition/chats-delete.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { pathRoute, toBody, type UseCase } from '../../shared/http/routes.js';

/** What the chats-delete handler needs. */
export interface ChatsDeleteHandlerDeps {
  readonly http: HttpDeps;
  readonly deleteChat: UseCase<[Principal, string], void>;
}

/**
 * Builds the chats-delete handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{deleted: true}`, 400, 401 or 404 `CHAT_NOT_FOUND`.
 */
export const createChatsDeleteHandler = (deps: ChatsDeleteHandlerDeps) =>
  pathRoute(deps.http, 'chats-delete', async (principal, id) =>
    toBody(await deps.deleteChat(principal, id), () => ({ deleted: true })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createChatsDeleteHandler(await chatsDeleteDeps()));
