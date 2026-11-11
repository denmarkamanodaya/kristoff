import { describe, expect, it } from 'vitest';
import { aChat } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { listChats } from './v1.js';

describe('list-chats v1', () => {
  it("lists the principal's chats newest first in pages of 50", async () => {
    const chats = memoryChatStore();
    for (let i = 0; i < 51; i += 1) {
      const at = new Date(Date.UTC(2026, 0, 1) + i * 1000).toISOString();
      await chats.store.createChat(
        'kristoff',
        aChat({ id: i.toString(16).padStart(16, '0'), updatedAt: at }),
      );
    }

    const page = await listChats({ chats: chats.store })({ uid: 'kristoff' }, undefined);

    expect(
      page.ok && [page.value.items.length, page.value.items[0]?.id, page.value.nextCursor],
    ).toEqual([50, (50).toString(16).padStart(16, '0'), '50']);
  });
});
