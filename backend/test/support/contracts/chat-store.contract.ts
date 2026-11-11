// The ChatStore contract suite: run against the in-memory fake and the DynamoDB adapter.
import { describe, expect, it } from 'vitest';
import type { ChatStore } from '../../../src/ports/chat-store.js';
import { aChat, aMessage } from '../builders.js';

/** Registers the suite for a store factory (each call returns an empty store). */
export const chatStoreContract = (name: string, make: () => Promise<ChatStore>) => {
  describe(`ChatStore contract: ${name}`, () => {
    it('creates a chat, reads it back under the owner only', async () => {
      const store = await make();
      await store.createChat('u1', aChat());
      expect(await store.getChat('u1', aChat().id)).toEqual({
        ok: true,
        value: { value: aChat(), version: 1 },
      });
      expect(await store.getChat('u2', aChat().id)).toMatchObject({
        error: { code: 'CHAT_NOT_FOUND' },
      });
    });

    it('lists chats newest updatedAt first and pages them', async () => {
      const store = await make();
      await store.createChat(
        'u1',
        aChat({ id: '0000000000000001', updatedAt: '2026-01-01T00:00:00.000Z' }),
      );
      await store.createChat(
        'u1',
        aChat({ id: '0000000000000002', updatedAt: '2026-01-03T00:00:00.000Z' }),
      );
      await store.createChat(
        'u1',
        aChat({ id: '0000000000000003', updatedAt: '2026-01-02T00:00:00.000Z' }),
      );
      const first = await store.listChats('u1', undefined, 2);
      expect(first.ok && first.value.items.map((c) => c.id)).toEqual([
        '0000000000000002',
        '0000000000000003',
      ]);
      const next = await store.listChats(
        'u1',
        (first.ok ? first.value.nextCursor : null) ?? undefined,
        2,
      );
      expect(next.ok && next.value.items.map((c) => c.id)).toEqual(['0000000000000001']);
    });

    it('updates at the read version, moving the chat to the top of the list', async () => {
      const store = await make();
      await store.createChat(
        'u1',
        aChat({ id: '0000000000000001', updatedAt: '2026-01-01T00:00:00.000Z' }),
      );
      await store.createChat(
        'u1',
        aChat({ id: '0000000000000002', updatedAt: '2026-01-02T00:00:00.000Z' }),
      );
      const updated = await store.updateChat(
        'u1',
        '0000000000000001',
        { title: 'T', updatedAt: '2026-01-05T00:00:00.000Z' },
        1,
      );
      expect(updated).toMatchObject({ ok: true, value: { title: 'T', docIds: [] } });
      const list = await store.listChats('u1', undefined, 5);
      expect(list.ok && list.value.items[0]?.id).toBe('0000000000000001');
      expect(await store.updateChat('u1', '0000000000000001', { updatedAt: 'x' }, 1)).toMatchObject(
        { error: { code: 'VERSION_CONFLICT' } },
      );
    });

    it('appends messages, refuses a duplicate id, returns the newest N oldest first', async () => {
      const store = await make();
      for (const n of [1, 2, 3])
        await store.putMessage(
          'c1',
          aMessage({ id: `01J${String(n).padStart(23, '0')}`, text: `m${String(n)}` }),
        );
      expect(
        (await store.putMessage('c1', aMessage({ id: `01J${'1'.padStart(23, '0')}` }))).ok,
      ).toBe(false);
      const recent = await store.recentMessages('c1', 2);
      expect(recent.ok && recent.value.map((m) => m.text)).toEqual(['m2', 'm3']);
      expect(await store.recentMessages('c1', 0)).toEqual({ ok: true, value: [] });
    });
  });
};
