import { describe, expect, it } from 'vitest';
import { ChatWithMessagesResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { aChat, aMessage } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createChatsGetHandler } from './v1.js';

const ROUTE = 'GET /v1/chats/{id}';

describe('chats-get handler v1', () => {
  it('answers {chat, messages} for the path id', async () => {
    const value = { chat: aChat(), messages: [aMessage()] };
    const handler = createChatsGetHandler({
      http: httpDeps(),
      getChat: () => Promise.resolve(ok(value)),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'cccccccccccccccc' } }));

    expect([res.statusCode, ChatWithMessagesResponse.parse(bodyOf(res))]).toEqual([200, value]);
  });

  it('answers 404 CHAT_NOT_FOUND', async () => {
    const handler = createChatsGetHandler({
      http: httpDeps(),
      getChat: () => Promise.resolve(err(appError('CHAT_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'cccccccccccccccc' } }))).statusCode,
    ).toBe(404);
  });
});
