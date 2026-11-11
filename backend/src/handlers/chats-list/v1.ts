// GET /v1/chats handler (docs/contract.md "Routes"): thin driving adapter.
import type { ChatSummary, Page, Principal } from '../../domain/model.js';
import type { HttpDeps } from '../../shared/http/http.js';
import { chatsListDeps } from '../../composition/chats-list.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import { type UseCase, listRoute, toBody } from '../../shared/http/routes.js';

/** What the chats-list handler needs. */
export interface ChatsListHandlerDeps {
  readonly http: HttpDeps;
  readonly listChats: UseCase<[Principal, string | undefined], Page<ChatSummary>>;
}

/**
 * Builds the chats-list handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Plumbing and the wired use case.
 * @returns The handler: 200 `{items, nextCursor}` newest first, 400 or 401.
 */
export const createChatsListHandler = (deps: ChatsListHandlerDeps) =>
  listRoute(deps.http, 'chats-list', async (principal, cursor) =>
    toBody(await deps.listChats(principal, cursor)),
  );

/** Lambda entry (`index.handler`): wires on the first invocation, once per container. */
export const handler = lazyHandler(async () => createChatsListHandler(await chatsListDeps()));
