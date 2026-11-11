import { describe, expect, it } from 'vitest';
import type { ChatStore } from '../../ports/chat-store.js';
import { appError } from '../../shared/errors/app-error.js';
import { err } from '../../shared/result/result.js';
import { aChat, aMessage } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { deleteChat } from './v1.js';

const ME = { uid: 'kristoff' };
const ID = aChat().id;

const withMessages = async () => {
  const chats = memoryChatStore();
  await chats.store.createChat('kristoff', aChat());
  await chats.store.putMessage(ID, aMessage({ id: `01J${'1'.padStart(23, '0')}` }));
  await chats.store.putMessage(ID, aMessage({ id: `01J${'2'.padStart(23, '0')}` }));
  return chats;
};

describe('delete-chat v1', () => {
  it('deletes every message, then the chat item, in that order', async () => {
    const chats = await withMessages();
    const calls: string[] = [];
    const store: ChatStore = {
      ...chats.store,
      deleteMessages: (id) => (calls.push(`messages:${id}`), chats.store.deleteMessages(id)),
      deleteChat: (uid, id) => (calls.push(`chat:${id}`), chats.store.deleteChat(uid, id)),
    };

    const result = await deleteChat({ chats: store })(ME, ID);

    expect(result).toEqual({ ok: true, value: undefined });
    expect(calls).toEqual([`messages:${ID}`, `chat:${ID}`]);
    expect([chats.chats.size, chats.messages.has(ID)]).toEqual([0, false]);
  });

  it('keeps the chat item when deleting messages fails, so the delete can be repeated', async () => {
    const chats = await withMessages();
    const store: ChatStore = {
      ...chats.store,
      deleteMessages: () => Promise.resolve(err(appError('UPSTREAM_UNAVAILABLE', 'throttled'))),
    };

    const result = await deleteChat({ chats: store })(ME, ID);

    expect(result).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(chats.chats.has(`kristoff/${ID}`)).toBe(true);
    expect(await deleteChat({ chats: chats.store })(ME, ID)).toEqual({
      ok: true,
      value: undefined,
    });
  });

  it("answers CHAT_NOT_FOUND for another owner's chat and touches none of its messages", async () => {
    const chats = await withMessages();

    const result = await deleteChat({ chats: chats.store })({ uid: 'eve' }, ID);

    expect(result).toMatchObject({ error: { code: 'CHAT_NOT_FOUND' } });
    expect(chats.messages.get(ID)).toHaveLength(2);
  });

  it('answers CHAT_NOT_FOUND on a repeat after success', async () => {
    const chats = await withMessages();
    await deleteChat({ chats: chats.store })(ME, ID);

    expect(await deleteChat({ chats: chats.store })(ME, ID)).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
  });
});
