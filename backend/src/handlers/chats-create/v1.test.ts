import { describe, expect, it } from 'vitest';
import { ChatResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aChat } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createChatsCreateHandler } from './v1.js';

describe('chats-create handler v1', () => {
  it('passes docIds (none when the body is empty) and answers {chat}', async () => {
    const seen: unknown[] = [];
    const handler = createChatsCreateHandler({
      http: httpDeps(),
      createChat: (p, ids) => (seen.push([p, ids]), Promise.resolve(ok(aChat()))),
    });

    const withIds = await handler(
      httpEvent({ routeKey: 'POST /v1/chats', body: { docIds: ['aaaaaaaaaaaaaaaa'] } }),
    );
    await handler(httpEvent({ routeKey: 'POST /v1/chats' }));

    expect([withIds.statusCode, ChatResponse.parse(bodyOf(withIds))]).toEqual([
      200,
      { chat: aChat() },
    ]);
    expect(seen).toEqual([
      [{ uid: 'kristoff' }, ['aaaaaaaaaaaaaaaa']],
      [{ uid: 'kristoff' }, []],
    ]);
  });

  it('answers 400 for more than 20 docIds or a malformed id', async () => {
    const handler = createChatsCreateHandler({
      http: httpDeps(),
      createChat: () => Promise.reject(new Error('called')),
    });
    const many = Array.from({ length: 21 }, (_, i) => i.toString(16).padStart(16, '0'));

    expect(
      (await handler(httpEvent({ routeKey: 'POST /v1/chats', body: { docIds: many } }))).statusCode,
    ).toBe(400);
    expect(
      (await handler(httpEvent({ routeKey: 'POST /v1/chats', body: { docIds: ['X'] } })))
        .statusCode,
    ).toBe(400);
  });
});
