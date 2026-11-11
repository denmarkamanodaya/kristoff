import { describe, expect, it } from 'vitest';
import { fakeClock, fakeIds } from '../../../test/support/fakes/clock-ids.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { createDocUpload } from './v1.js';

const setup = () => {
  const docs = memoryDocStore();
  const files = memoryFileStore();
  const run = createDocUpload({
    docs: docs.store,
    files: files.store,
    ids: fakeIds(),
    clock: fakeClock(),
  });
  return { docs, files, run };
};

describe('create-doc-upload v1', () => {
  it('stores a pending doc under the owner and presigns a PUT on the server-built key', async () => {
    const { docs, files, run } = setup();

    const result = await run({ uid: 'kristoff' }, { fileName: 'People v. Cruz.PDF', size: 1234 });

    expect(result).toEqual({
      ok: true,
      value: {
        doc: {
          id: '0000000000000001',
          title: 'People v. Cruz',
          size: 1234,
          status: 'pending',
          createdAt: '2026-11-12T01:00:00.000Z',
        },
        uploadUrl: 'https://put.example/docs/kristoff/0000000000000001.pdf',
        uploadHeaders: { 'Content-Type': 'application/pdf' },
      },
    });
    expect(docs.rows.get('kristoff/0000000000000001')?.objectKey).toBe(
      'docs/kristoff/0000000000000001.pdf',
    );
    expect(files.presigned).toEqual([
      { method: 'PUT', key: 'docs/kristoff/0000000000000001.pdf', ttl: 300, size: 1234 },
    ]);
  });

  it('titles a blank name "Document" and clamps long names to 200 characters', async () => {
    const { run } = setup();

    const blank = await run({ uid: 'kristoff' }, { fileName: '.pdf', size: 1 });
    const long = await run({ uid: 'kristoff' }, { fileName: `${'x'.repeat(250)}.pdf`, size: 1 });

    expect(blank.ok && blank.value.doc.title).toBe('Document');
    expect(long.ok && long.value.doc.title.length).toBe(200);
  });
});
