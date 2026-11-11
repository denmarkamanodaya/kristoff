import { describe, expect, it } from 'vitest';
import type { DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import { appError } from '../../shared/errors/app-error.js';
import { err } from '../../shared/result/result.js';
import { aChat, aDoc } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { deleteDoc } from './v1.js';

const D = 'aaaaaaaaaaaaaaaa';
const OTHER = 'bbbbbbbbbbbbbbbb';
const ME = { uid: 'kristoff' };

const setup = async () => {
  const docs = memoryDocStore();
  const chats = memoryChatStore();
  const files = memoryFileStore();
  await docs.store.create('kristoff', aDoc({ id: D }), 'k');
  await chats.store.createChat(
    'kristoff',
    aChat({ docIds: [D, OTHER], updatedAt: '2026-01-01T00:00:00.000Z' }),
  );
  await chats.store.createChat(
    'kristoff',
    aChat({ id: '0000000000000002', docIds: [`${D}:p1`, `${D}:p2`] }),
  );
  await chats.store.createChat('kristoff', aChat({ id: '0000000000000003', docIds: [OTHER] }));
  files.put(`docs/kristoff/${D}.pdf`, '%PDF-');
  files.put(`docs/kristoff/${D}/part-1.pdf`, '%PDF-');
  files.put(`docs/kristoff/${OTHER}.pdf`, '%PDF-');
  return { docs, chats, files };
};

describe('delete-doc v1', () => {
  it('unlinks the doc and its parts from every chat (keeping updatedAt), purges both prefixes, then removes the item', async () => {
    const { docs, chats, files } = await setup();
    const calls: string[] = [];
    const fileStore: FileStore = {
      ...files.store,
      purgePrefix: (p) => (calls.push(`purge:${p}`), files.store.purgePrefix(p)),
    };
    const docStore: DocStore = {
      ...docs.store,
      remove: (u, id) => (calls.push(`remove:${id}`), docs.store.remove(u, id)),
    };

    const result = await deleteDoc({ docs: docStore, chats: chats.store, files: fileStore })(ME, D);

    expect(result).toEqual({ ok: true, value: undefined });
    expect(calls).toEqual([
      `purge:docs/kristoff/${D}.pdf`,
      `purge:docs/kristoff/${D}/`,
      `remove:${D}`,
    ]);
    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat).toMatchObject({
      docIds: [OTHER],
      updatedAt: '2026-01-01T00:00:00.000Z',
    });
    expect(chats.chats.get('kristoff/0000000000000002')?.chat.docIds).toEqual([]);
    expect([...files.objects.keys()]).toEqual([`docs/kristoff/${OTHER}.pdf`]);
    expect(docs.rows.size).toBe(0);
  });

  it('keeps the doc item when the purge fails, so the delete can be repeated', async () => {
    const { docs, chats, files } = await setup();
    const failing: FileStore = {
      ...files.store,
      purgePrefix: () => Promise.resolve(err(appError('UPSTREAM_UNAVAILABLE', 's3'))),
    };

    expect(
      await deleteDoc({ docs: docs.store, chats: chats.store, files: failing })(ME, D),
    ).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(docs.rows.size).toBe(1);
    expect(
      (await deleteDoc({ docs: docs.store, chats: chats.store, files: files.store })(ME, D)).ok,
    ).toBe(true);
  });

  it("answers DOC_NOT_FOUND for another owner's doc without touching anything, and on a repeat", async () => {
    const { docs, chats, files } = await setup();
    const run = deleteDoc({ docs: docs.store, chats: chats.store, files: files.store });

    expect(await run({ uid: 'eve' }, D)).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
    expect(files.objects.size).toBe(3);
    await run(ME, D);
    expect(await run(ME, D)).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
  });
});
