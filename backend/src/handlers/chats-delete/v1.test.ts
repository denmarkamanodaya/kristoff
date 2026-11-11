import { describe, expect, it } from 'vitest';
import { DeletedResponse } from '../../contracts/shapes.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { bodyOf, httpDeps, httpEvent } from '../../../test/support/http-events.js';
import { createChatsDeleteHandler } from './v1.js';

const ROUTE = 'DELETE /v1/chats/{id}';

describe('chats-delete handler v1', () => {
  it('passes the principal and path id and answers {deleted: true}', async () => {
    const seen: unknown[] = [];
    const handler = createChatsDeleteHandler({
      http: httpDeps(),
      deleteChat: (p, id) => (seen.push([p, id]), Promise.resolve(ok(undefined))),
    });

    const res = await handler(httpEvent({ routeKey: ROUTE, path: { id: 'cccccccccccccccc' } }));

    expect([res.statusCode, DeletedResponse.parse(bodyOf(res))]).toEqual([200, { deleted: true }]);
    expect(seen).toEqual([[{ uid: 'kristoff' }, 'cccccccccccccccc']]);
  });

  it('answers 404 CHAT_NOT_FOUND and 400 for a malformed id', async () => {
    const handler = createChatsDeleteHandler({
      http: httpDeps(),
      deleteChat: () => Promise.resolve(err(appError('CHAT_NOT_FOUND', 'no'))),
    });

    expect(
      (await handler(httpEvent({ routeKey: ROUTE, path: { id: 'cccccccccccccccc' } }))).statusCode,
    ).toBe(404);
    expect((await handler(httpEvent({ routeKey: ROUTE, path: { id: 'x' } }))).statusCode).toBe(400);
  });
});
