import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { renameDoc } from './v1.js';

describe('rename-doc v1', () => {
  it('stores the trimmed title', async () => {
    const docs = memoryDocStore();
    await docs.store.create('kristoff', aDoc(), 'k');

    const result = await renameDoc({ docs: docs.store })({ uid: 'kristoff' }, aDoc().id, '  Cruz ');

    expect(result).toMatchObject({ ok: true, value: { title: 'Cruz' } });
  });

  it('refuses a blank title and an unknown doc', async () => {
    const docs = memoryDocStore();
    const run = renameDoc({ docs: docs.store });

    expect(await run({ uid: 'kristoff' }, aDoc().id, '   ')).toMatchObject({
      ok: false,
      error: { code: 'VALIDATION_FAILED' },
    });
    expect(await run({ uid: 'kristoff' }, aDoc().id, 'x')).toMatchObject({
      ok: false,
      error: { code: 'DOC_NOT_FOUND' },
    });
  });
});
