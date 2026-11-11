import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { getDocLink } from './v1.js';

describe('get-doc-link v1', () => {
  it("presigns a 300 s GET on the ready doc's object key", async () => {
    const docs = memoryDocStore();
    const files = memoryFileStore(() => Date.UTC(2026, 10, 12));
    await docs.store.create('kristoff', aDoc(), 'k');

    const result = await getDocLink({ docs: docs.store, files: files.store })(
      { uid: 'kristoff' },
      aDoc().id,
      undefined,
    );

    expect(result).toEqual({
      ok: true,
      value: {
        url: 'https://get.example/docs/kristoff/aaaaaaaaaaaaaaaa.pdf',
        expiresAt: '2026-11-12T00:05:00.000Z',
      },
    });
  });

  it("answers DOC_NOT_FOUND for a pending doc and for another owner's doc", async () => {
    const docs = memoryDocStore();
    await docs.store.create('kristoff', aDoc({ status: 'pending' }), 'k');
    await docs.store.create('eve', aDoc({ id: 'eeeeeeeeeeeeeeee' }), 'k');
    const run = getDocLink({ docs: docs.store, files: memoryFileStore().store });

    expect(await run({ uid: 'kristoff' }, aDoc().id, undefined)).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
    expect(await run({ uid: 'kristoff' }, 'eeeeeeeeeeeeeeee', undefined)).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
  });

  it('links an existing part of a split doc, and refuses a part that does not exist', async () => {
    const docs = memoryDocStore();
    const parts = [
      { n: 1, startPage: 1, endPage: 100, size: 9 },
      { n: 2, startPage: 101, endPage: 150, size: 9 },
    ];
    await docs.store.create('kristoff', aDoc({ parts }), 'k');
    const run = getDocLink({ docs: docs.store, files: memoryFileStore().store });

    expect(await run({ uid: 'kristoff' }, aDoc().id, 2)).toMatchObject({
      ok: true,
      value: { url: 'https://get.example/docs/kristoff/aaaaaaaaaaaaaaaa/part-2.pdf' },
    });
    expect(await run({ uid: 'kristoff' }, aDoc().id, 3)).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
  });
});
