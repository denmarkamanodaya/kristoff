import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { aChat } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { eachOwnerChat } from './v1.js';

const setup = async () => {
  const chats = memoryChatStore();
  for (let i = 0; i < 120; i += 1)
    await chats.store.createChat(
      'kristoff',
      aChat({ id: i.toString(16).padStart(16, '0'), pinned: i % 3 === 0 }),
    );
  await chats.store.createChat('eve', aChat({ id: 'eeeeeeeeeeeeeeee', pinned: true }));
  return chats;
};

describe('each-owner-chat v1', () => {
  it("visits every selected chat of the owner across pages, and no one else's", async () => {
    const chats = await setup();
    const seen: string[] = [];

    const result = await eachOwnerChat(chats.store, 'kristoff', {
      select: (c) => c.pinned,
      visit: (c) => (seen.push(c.id), Promise.resolve(ok(undefined))),
    });

    expect([result.ok, seen.length, seen.includes('eeeeeeeeeeeeeeee')]).toEqual([true, 40, false]);
  });

  it('stops at the first failed visit', async () => {
    const chats = await setup();
    let visits = 0;

    const result = await eachOwnerChat(chats.store, 'kristoff', {
      select: () => true,
      visit: () => ((visits += 1), Promise.resolve(err(appError('VERSION_CONFLICT', 'x')))),
    });

    expect([result.ok, visits]).toEqual([false, 1]);
  });
});
