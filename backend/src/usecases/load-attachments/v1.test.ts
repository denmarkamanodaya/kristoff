import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { loadAttachments } from './v1.js';

const A = 'aaaaaaaaaaaaaaaa';
const B = 'bbbbbbbbbbbbbbbb';

describe('load-attachments v1', () => {
  it('resolves each id in order, undefined where it is not usable', async () => {
    const docs = memoryDocStore();
    const parts = [{ n: 1, startPage: 1, endPage: 100, size: 9 }];
    await docs.store.create('kristoff', aDoc({ id: A }), 'k');
    await docs.store.create('kristoff', aDoc({ id: B, parts }), 'k');

    const result = await loadAttachments(docs.store, 'kristoff', [
      A,
      `${B}:p1`,
      B,
      'junk',
      `${B}:p9`,
    ]);

    expect(result.ok && result.value.map((t) => t?.kind)).toEqual([
      'whole',
      'part',
      undefined,
      undefined,
      undefined,
    ]);
  });

  it("reads nothing for no ids or only malformed ones, and never another owner's doc", async () => {
    const docs = memoryDocStore();
    await docs.store.create('eve', aDoc({ id: A }), 'k');

    expect(await loadAttachments(docs.store, 'kristoff', [])).toEqual({ ok: true, value: [] });
    expect(await loadAttachments(docs.store, 'kristoff', ['x'])).toEqual({
      ok: true,
      value: [undefined],
    });
    expect(await loadAttachments(docs.store, 'kristoff', [A])).toEqual({
      ok: true,
      value: [undefined],
    });
  });
});
