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

    expect(await run({ uid: 'kristoff' }, aDoc().id)).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
    expect(await run({ uid: 'kristoff' }, 'eeeeeeeeeeeeeeee')).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
  });
});
