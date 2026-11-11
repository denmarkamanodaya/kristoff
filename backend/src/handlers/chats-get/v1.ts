// GET /v1/chats/{id} handler (docs/contract.md "Routes"): thin driving adapter.
import type { Principal } from '../../domain/model.js';
import type { ChatWithMessages } from '../../usecases/get-chat/index.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { chatsGetDeps } from '../../composition/chats-get.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, pathRoute, toBody } from '../../shared/http/routes.js';

/** What the chats-get handler needs. */
export interface ChatsGetHandlerDeps {
  readonly http: HttpDeps;
  readonly getChat: UseCase<[Principal, string], ChatWithMessages>;
}

/**
 * Builds the chats-get handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{chat, messages}`, 400, 401 or 404 `CHAT_NOT_FOUND`.
 */
export const createChatsGetHandler = (deps: ChatsGetHandlerDeps) =>
  pathRoute(deps.http, 'chats-get', async (principal, id) =>
    toBody(await deps.getChat(principal, id)),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createChatsGetHandler(await chatsGetDeps()));
