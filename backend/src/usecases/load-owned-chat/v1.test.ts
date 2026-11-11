import { describe, expect, it } from 'vitest';
import { aChat } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { ok } from '../../shared/result/result.js';
import { forOwnedChat, loadOwnedChat } from './v1.js';

describe('load-owned-chat v1', () => {
  it("returns the principal's chat with its version", async () => {
    const chats = memoryChatStore();
    await chats.store.createChat('kristoff', aChat());

    expect(await loadOwnedChat(chats.store, { uid: 'kristoff' }, aChat().id)).toEqual({
      ok: true,
      value: { value: aChat(), version: 1 },
    });
  });

  it("answers CHAT_NOT_FOUND for another owner's chat and for a missing one", async () => {
    const chats = memoryChatStore();
    await chats.store.createChat('eve', aChat());

    expect(await loadOwnedChat(chats.store, { uid: 'kristoff' }, aChat().id)).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
    expect(await loadOwnedChat(chats.store, { uid: 'eve' }, 'ffffffffffffffff')).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
  });

  it('runs the action with the loaded chat only when the principal owns it', async () => {
    const chats = memoryChatStore();
    await chats.store.createChat('kristoff', aChat());
    const seen: unknown[] = [];
    const run = forOwnedChat(chats.store, (owned, p, id) => {
      seen.push([owned.version, p, id]);
      return Promise.resolve(ok('done'));
    });

    const mine = await run({ uid: 'kristoff' }, aChat().id);
    const theirs = await run({ uid: 'eve' }, aChat().id);

    expect(mine).toEqual({ ok: true, value: 'done' });
    expect(theirs).toMatchObject({ error: { code: 'CHAT_NOT_FOUND' } });
    expect(seen).toEqual([[1, { uid: 'kristoff' }, aChat().id]]);
  });
});
