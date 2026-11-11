// PATCH /v1/chats/{id} handler (docs/contract.md "Routes"): thin driving adapter.
import type { Chat, Principal } from '../../domain/model.js';
import type { ChatChanges } from '../../usecases/update-chat/index.js';
import { UpdateChatRequest } from '../../contracts/requests.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { chatsUpdateDeps } from '../../composition/chats-update.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, pathBodyRoute, toBody } from '../../shared/http/routes.js';

/** What the chats-update handler needs. */
export interface ChatsUpdateHandlerDeps {
  readonly http: HttpDeps;
  readonly updateChat: UseCase<[Principal, string, ChatChanges], Chat>;
}

/**
 * Builds the chats-update handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{chat}`, 400, 401, 404 `CHAT_NOT_FOUND` or `DOC_NOT_FOUND`.
 */
export const createChatsUpdateHandler = (deps: ChatsUpdateHandlerDeps) =>
  pathBodyRoute(deps.http, 'chats-update', UpdateChatRequest, async (principal, id, body) => {
    const { title, docIds } = body;
    const changes = {
      ...(title === undefined ? {} : { title }),
      ...(docIds === undefined ? {} : { docIds }),
    };
    return toBody(await deps.updateChat(principal, id, changes), (chat) => ({ chat }));
  });

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createChatsUpdateHandler(await chatsUpdateDeps()));
