import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { fakeClock, fakeIds } from '../../../test/support/fakes/clock-ids.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { createChat } from './v1.js';

describe('create-chat v1', () => {
  it('creates "New chat" with the resolved doc ids at the clock instant', async () => {
    const chats = memoryChatStore();
    const seen: string[][] = [];
    const run = createChat({
      chats: chats.store,
      ids: fakeIds(),
      clock: fakeClock(),
      resolveDocs: (_p, ids) => {
        seen.push([...ids]);
        return Promise.resolve(ok(['aaaaaaaaaaaaaaaa']));
      },
    });

    const result = await run({ uid: 'kristoff' }, ['aaaaaaaaaaaaaaaa', 'aaaaaaaaaaaaaaaa']);

    expect(result).toEqual({
      ok: true,
      value: {
        id: '0000000000000001',
        title: 'New chat',
        docIds: ['aaaaaaaaaaaaaaaa'],
        pinned: false,
        projectId: null,
        createdAt: '2026-11-12T01:00:00.000Z',
        updatedAt: '2026-11-12T01:00:00.000Z',
      },
    });
    expect(seen).toEqual([['aaaaaaaaaaaaaaaa', 'aaaaaaaaaaaaaaaa']]);
    expect(chats.chats.has('kristoff/0000000000000001')).toBe(true);
  });

  it('creates nothing when the docs check fails', async () => {
    const chats = memoryChatStore();
    const run = createChat({
      chats: chats.store,
      ids: fakeIds(),
      clock: fakeClock(),
      resolveDocs: () => Promise.resolve(err(appError('DOC_NOT_FOUND', 'x'))),
    });

    expect(await run({ uid: 'kristoff' }, ['x'])).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
    expect(chats.chats.size).toBe(0);
  });
});
