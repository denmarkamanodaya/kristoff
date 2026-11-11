import { describe, expect, it } from 'vitest';
import { aChat, aMessage } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { getChat } from './v1.js';

describe('get-chat v1', () => {
  it('returns the chat with its newest 200 messages, oldest first', async () => {
    const chats = memoryChatStore();
    await chats.store.createChat('kristoff', aChat());
    for (let i = 0; i < 201; i += 1)
      await chats.store.putMessage(
        aChat().id,
        aMessage({ id: `01J${String(i).padStart(23, '0')}` }),
      );

    const result = await getChat({ chats: chats.store })({ uid: 'kristoff' }, aChat().id);

    expect(result.ok && result.value.chat).toEqual(aChat());
    expect(result.ok && result.value.messages.length).toBe(200);
    expect(result.ok && result.value.messages[0]?.id).toBe(`01J${'1'.padStart(23, '0')}`);
  });

  it("answers CHAT_NOT_FOUND for another owner's chat without reading its messages", async () => {
    const chats = memoryChatStore();
    await chats.store.createChat('eve', aChat());

    expect(await getChat({ chats: chats.store })({ uid: 'kristoff' }, aChat().id)).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
  });
});
