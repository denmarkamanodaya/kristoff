import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { listDocs } from './v1.js';

describe('list-docs v1', () => {
  it("lists only the principal's docs in pages of 100", async () => {
    const docs = memoryDocStore();
    for (let i = 0; i < 101; i += 1)
      await docs.store.create('kristoff', aDoc({ id: i.toString(16).padStart(16, '0') }), 'k');
    await docs.store.create('eve', aDoc({ id: 'eeeeeeeeeeeeeeee' }), 'k');

    const first = await listDocs({ docs: docs.store })({ uid: 'kristoff' }, undefined);
    const second = await listDocs({ docs: docs.store })({ uid: 'kristoff' }, '100');

    expect(first.ok && [first.value.items.length, first.value.nextCursor]).toEqual([100, '100']);
    expect(second.ok && [second.value.items.length, second.value.nextCursor]).toEqual([1, null]);
  });
});
