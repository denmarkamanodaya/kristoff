import { describe, expect, it } from 'vitest';
import type { DocSplitTrigger } from '../../ports/doc-split-trigger.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { confirmDoc } from './v1.js';

const KEY = 'docs/kristoff/aaaaaaaaaaaaaaaa.pdf';
const ME = { uid: 'kristoff' };

const recordingTrigger = (result: Result<void, AppError> = ok(undefined)) => {
  const started: string[] = [];
  const trigger: DocSplitTrigger = {
    start: (uid, docId) => (started.push(`${uid}/${docId}`), Promise.resolve(result)),
  };
  return { trigger, started };
};

const setup = async (size = 9, trigger = recordingTrigger()) => {
  const docs = memoryDocStore();
  const files = memoryFileStore();
  await docs.store.create('kristoff', aDoc({ status: 'pending', size }), KEY);
  const run = confirmDoc({ docs: docs.store, files: files.store, splits: trigger.trigger });
  return { docs, files, run, started: trigger.started };
};

describe('confirm-doc v1', () => {
  it('marks a verified PDF processing and starts docs-split for it (every PDF, small ones too)', async () => {
    const { files, run, started } = await setup();
    files.put(KEY, '%PDF-1.7\n');

    const result = await run(ME, 'aaaaaaaaaaaaaaaa');

    expect(result).toMatchObject({ ok: true, value: { status: 'processing' } });
    expect(started).toEqual(['kristoff/aaaaaaaaaaaaaaaa']);
  });

  it('marks the doc failed and returns the error when docs-split cannot be started', async () => {
    const { docs, files, run } = await setup(
      9,
      recordingTrigger(err(appError('UPSTREAM_UNAVAILABLE', 'lambda'))),
    );
    files.put(KEY, '%PDF-1.7\n');

    const result = await run(ME, 'aaaaaaaaaaaaaaaa');

    expect(result).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(docs.rows.get('kristoff/aaaaaaaaaaaaaaaa')?.doc.status).toBe('failed');
  });

  it.each(['processing', 'ready', 'failed'] as const)(
    'returns a %s doc unchanged without starting another split',
    async (status) => {
      const docs = memoryDocStore();
      const trigger = recordingTrigger();
      await docs.store.create('kristoff', aDoc({ status }), KEY);
      const run = confirmDoc({
        docs: docs.store,
        files: memoryFileStore().store,
        splits: trigger.trigger,
      });

      expect(await run(ME, 'aaaaaaaaaaaaaaaa')).toMatchObject({ ok: true, value: { status } });
      expect(trigger.started).toEqual([]);
    },
  );

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

  it('answers DOC_NOT_FOUND for another id', async () => {
    const { run } = await setup();

    expect(await run(ME, 'ffffffffffffffff')).toMatchObject({
      ok: false,
      error: { code: 'DOC_NOT_FOUND' },
    });
  });
});
