import { describe, expect, it } from 'vitest';
import { ChatResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aChat } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createChatsUpdateHandler } from './v1.js';

const ROUTE = 'PATCH /v1/chats/{id}';
const path = { id: 'cccccccccccccccc' };

describe('chats-update handler v1', () => {
  it('passes only the fields sent and answers {chat}', async () => {
    const seen: unknown[] = [];
    const handler = createChatsUpdateHandler({
      http: httpDeps(),
      updateChat: (p, id, c) => (seen.push([p, id, c]), Promise.resolve(ok(aChat()))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path, body: { title: 'T' } }));
    await handler(httpEvent({ routeKey: ROUTE, path, body: { docIds: [] } }));

    expect([res.statusCode, ChatResponse.parse(bodyOf(res))]).toEqual([200, { chat: aChat() }]);
    expect(seen).toEqual([
      [{ uid: 'kristoff' }, 'cccccccccccccccc', { title: 'T' }],
      [{ uid: 'kristoff' }, 'cccccccccccccccc', { docIds: [] }],
    ]);
  });

  it('answers 400 for an empty body (at least one field) and a 121-char title', async () => {
    const handler = createChatsUpdateHandler({
      http: httpDeps(),
      updateChat: () => Promise.reject(new Error('called')),
    });

    expect((await handler(httpEvent({ routeKey: ROUTE, path, body: {} }))).statusCode).toBe(400);
    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path, body: { title: 'x'.repeat(121) } })))
        .statusCode,
    ).toBe(400);
  });
});
