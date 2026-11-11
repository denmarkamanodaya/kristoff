import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { confirmDoc } from './v1.js';

const KEY = 'docs/kristoff/aaaaaaaaaaaaaaaa.pdf';
const ME = { uid: 'kristoff' };

const setup = async (size = 9) => {
  const docs = memoryDocStore();
  const files = memoryFileStore();
  await docs.store.create('kristoff', aDoc({ status: 'pending', size }), KEY);
  return { docs, files, run: confirmDoc({ docs: docs.store, files: files.store }) };
};

describe('confirm-doc v1', () => {
  it('marks the doc ready when the object exists, matches the size and starts with %PDF-', async () => {
    const { files, run } = await setup();
    files.put(KEY, '%PDF-1.7\n');

    const result = await run(ME, 'aaaaaaaaaaaaaaaa');

    expect(result).toMatchObject({ ok: true, value: { status: 'ready' } });
  });

  it.each([
    ['no object', undefined],
    ['a size mismatch', '%PDF-1.7\n\n'],
    ['bytes that are not a PDF', 'GIF89a...'],
  ])('answers UPLOAD_INVALID for %s', async (_label, bytes) => {
    const { files, run } = await setup();
    if (bytes !== undefined) files.put(KEY, bytes);

    expect(await run(ME, 'aaaaaaaaaaaaaaaa')).toMatchObject({
      ok: false,
      error: { code: 'UPLOAD_INVALID' },
    });
  });

  it('returns an already ready doc unchanged without touching the bucket', async () => {
    const docs = memoryDocStore();
    await docs.store.create('kristoff', aDoc(), KEY);
    const run = confirmDoc({ docs: docs.store, files: memoryFileStore().store });

    expect(await run(ME, 'aaaaaaaaaaaaaaaa')).toMatchObject({ ok: true, value: aDoc() });
  });

  it('answers DOC_NOT_FOUND for another id', async () => {
    const { run } = await setup();

    expect(await run(ME, 'ffffffffffffffff')).toMatchObject({
      ok: false,
      error: { code: 'DOC_NOT_FOUND' },
    });
  });
});
