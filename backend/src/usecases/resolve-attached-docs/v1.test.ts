import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { resolveAttachedDocs } from './v1.js';

const A = 'aaaaaaaaaaaaaaaa';
const B = 'bbbbbbbbbbbbbbbb';
const ME = { uid: 'kristoff' };

const setup = async () => {
  const docs = memoryDocStore();
  await docs.store.create('kristoff', aDoc({ id: A }), 'k');
  await docs.store.create('kristoff', aDoc({ id: B, status: 'pending' }), 'k');
  return resolveAttachedDocs({ docs: docs.store, maxAttachedDocs: 2 });
};

describe('resolve-attached-docs v1', () => {
  it('returns ready ids de-duplicated in request order, and [] for none', async () => {
    const run = await setup();

    expect(await run(ME, [A, A])).toEqual({ ok: true, value: [A] });
    expect(await run(ME, [])).toEqual({ ok: true, value: [] });
  });

  it('answers DOC_NOT_FOUND for a pending or unknown doc', async () => {
    const run = await setup();

    expect(await run(ME, [A, B])).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
    expect(await run(ME, ['ffffffffffffffff'])).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
  });

  it('answers VALIDATION_FAILED above the configured cap', async () => {
    const run = await setup();

    expect(await run(ME, [A, B, 'cccccccccccccccc'])).toMatchObject({
      error: { code: 'VALIDATION_FAILED' },
    });
  });

  it('accepts a part id of a split doc but not the split doc whole', async () => {
    const docs = memoryDocStore();
    const parts = [{ n: 1, startPage: 1, endPage: 100, size: 9 }];
    await docs.store.create('kristoff', aDoc({ id: A, parts }), 'k');
    const run = resolveAttachedDocs({ docs: docs.store, maxAttachedDocs: 5 });

    expect(await run(ME, [`${A}:p1`, `${A}:p1`])).toEqual({ ok: true, value: [`${A}:p1`] });
    expect(await run(ME, [A])).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
    expect(await run(ME, [`${A}:p2`])).toMatchObject({ error: { code: 'DOC_NOT_FOUND' } });
  });
});
