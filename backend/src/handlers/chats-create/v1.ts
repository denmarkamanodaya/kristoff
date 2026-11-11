// POST /v1/chats handler (docs/contract.md "Routes"): thin driving adapter.
import type { Chat, Principal } from '../../domain/model.js';
import { CreateChatRequest } from '../../contracts/requests.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { chatsCreateDeps } from '../../composition/chats-create.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, bodyRoute, toBody } from '../../shared/http/routes.js';

/** What the chats-create handler needs. */
export interface ChatsCreateHandlerDeps {
  readonly http: HttpDeps;
  readonly createChat: UseCase<[Principal, readonly string[]], Chat>;
}

/**
 * Builds the chats-create handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{chat}`, 400, 401 or 404 `DOC_NOT_FOUND`.
 */
export const createChatsCreateHandler = (deps: ChatsCreateHandlerDeps) =>
  bodyRoute(deps.http, 'chats-create', CreateChatRequest, async (principal, body) =>
    toBody(await deps.createChat(principal, body.docIds ?? []), (chat) => ({ chat })),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createChatsCreateHandler(await chatsCreateDeps()));
