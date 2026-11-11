import { describe, expect, it } from 'vitest';
import { ChatsPageResponse } from '../../contracts/shapes.js';
import { ok } from '../../shared/result/result.js';
import { aChat } from '../../../test/support/builders.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createChatsListHandler } from './v1.js';

describe('chats-list handler v1', () => {
  it('passes the cursor and answers the page of summaries', async () => {
    const { id, title, docIds, updatedAt, pinned, projectId } = aChat();
    const page = { items: [{ id, title, docIds, updatedAt, pinned, projectId }], nextCursor: null };
    const seen: unknown[] = [];
    const handler = createChatsListHandler({
      http: httpDeps(),
      listChats: (p, c) => (seen.push([p, c]), Promise.resolve(ok(page))),
    });

    const res = await handler(httpEvent({ routeKey: 'GET /v1/chats' }));

    expect([res.statusCode, ChatsPageResponse.parse(bodyOf(res)), seen]).toEqual([
      200,
      page,
      [[{ uid: 'kristoff' }, undefined]],
    ]);
  });

  it('answers 400 for an over-long cursor', async () => {
    const handler = createChatsListHandler({
      http: httpDeps(),
      listChats: () => Promise.reject(new Error('called')),
    });

    expect(
      (await handler(httpEvent({ routeKey: 'GET /v1/chats', query: { cursor: 'x'.repeat(513) } })))
        .statusCode,
    ).toBe(400);
  });
});
