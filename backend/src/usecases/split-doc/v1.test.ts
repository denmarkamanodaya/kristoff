import { describe, expect, it } from 'vitest';
import type { FileStore } from '../../ports/file-store.js';
import { appError } from '../../shared/errors/app-error.js';
import { err } from '../../shared/result/result.js';
import { aDoc } from '../../../test/support/builders.js';
import { fakePdf, fakePdfReader } from '../../../test/support/fakes/fake-pdf-reader.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { splitDoc } from './v1.js';

const D = 'aaaaaaaaaaaaaaaa';
const KEY = `docs/kristoff/${D}.pdf`;
const MIB = 1024 * 1024;

const setup = async (
  pdf: Uint8Array,
  bytesPerPage = 1000,
  status: 'processing' | 'ready' = 'processing',
) => {
  const docs = memoryDocStore();
  const files = memoryFileStore();
  const pdfs = fakePdfReader(bytesPerPage);
  await docs.store.create('kristoff', aDoc({ id: D, status, pageCount: null }), KEY);
  files.put(KEY, pdf);
  return {
    docs,
    files,
    pdfs,
    run: splitDoc({ docs: docs.store, files: files.store, pdfs: pdfs.reader }),
  };
};

describe('split-doc v1', () => {
  it('marks a ≤ 100-page, ≤ 30 MiB PDF ready whole with its page count and no parts', async () => {
    const { docs, files, run } = await setup(fakePdf(100, 30 * MIB));

    expect(await run('kristoff', D)).toEqual({ ok: true, value: 'ready' });
    expect(docs.rows.get(`kristoff/${D}`)?.doc).toMatchObject({
      status: 'ready',
      pageCount: 100,
      parts: [],
    });
    expect(files.objects.size).toBe(1);
  });

  it('writes parts of ≤ 100 pages for a long PDF, in page order', async () => {
    const { docs, files, run } = await setup(fakePdf(250, 1000));

    await run('kristoff', D);

    expect(docs.rows.get(`kristoff/${D}`)?.doc.parts).toEqual([
      { n: 1, startPage: 1, endPage: 100, size: 100_000 },
      { n: 2, startPage: 101, endPage: 200, size: 100_000 },
      { n: 3, startPage: 201, endPage: 250, size: 50_000 },
    ]);
    expect([...files.objects.keys()].filter((k) => k.includes('/part-'))).toEqual([
      `docs/kristoff/${D}/part-1.pdf`,
      `docs/kristoff/${D}/part-2.pdf`,
      `docs/kristoff/${D}/part-3.pdf`,
    ]);
  });

  it('halves a range whose part would be over 30 MiB until every part fits', async () => {
    const { docs, pdfs, run } = await setup(fakePdf(80, 60 * MIB), MIB);

    await run('kristoff', D);

    expect(docs.rows.get(`kristoff/${D}`)?.doc.parts.map((p) => [p.startPage, p.endPage])).toEqual([
      [1, 20],
      [21, 40],
      [41, 60],
      [61, 80],
    ]);
    expect(pdfs.extracted.slice(0, 3)).toEqual([
      [1, 80],
      [1, 40],
      [1, 20],
    ]);
  });

  it('marks the doc failed when one page alone is over 30 MiB', async () => {
    const { docs, run } = await setup(fakePdf(2, 80 * MIB), 40 * MIB);

    expect(await run('kristoff', D)).toMatchObject({ error: { code: 'UPLOAD_INVALID' } });
    expect(docs.rows.get(`kristoff/${D}`)?.doc.status).toBe('failed');
  });

  it('marks the doc failed for an unreadable PDF and for a failed part write', async () => {
    const bad = await setup(new Uint8Array([1, 2, 3]));
    const ok = await setup(fakePdf(150, 1000));
    const failing: FileStore = {
      ...ok.files.store,
      put: () => Promise.resolve(err(appError('UPSTREAM_UNAVAILABLE', 's3'))),
    };
    const run = splitDoc({ docs: ok.docs.store, files: failing, pdfs: ok.pdfs.reader });

    expect(await bad.run('kristoff', D)).toMatchObject({ error: { code: 'UPLOAD_INVALID' } });
    expect(bad.docs.rows.get(`kristoff/${D}`)?.doc.status).toBe('failed');
    expect(await run('kristoff', D)).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(ok.docs.rows.get(`kristoff/${D}`)?.doc.status).toBe('failed');
  });

  it('skips a doc that is not processing (repeat delivery) or no longer exists', async () => {
    const { pdfs, run } = await setup(fakePdf(250, 1000), 1000, 'ready');

    expect(await run('kristoff', D)).toEqual({ ok: true, value: 'skipped' });
    expect(await run('kristoff', 'ffffffffffffffff')).toEqual({ ok: true, value: 'skipped' });
    expect(pdfs.extracted).toEqual([]);
  });

  it('keeps a rename made while processing (writes at the version read at the end)', async () => {
    const { docs, files, pdfs } = await setup(fakePdf(150, 1000));
    const renaming: FileStore = {
      ...files.store,
      put: async (key, bytes) => {
        const current = await docs.store.get('kristoff', D);
        if (current.ok && current.value.value.title !== 'Renamed')
          await docs.store.update('kristoff', D, { title: 'Renamed' }, current.value.version);
        return files.store.put(key, bytes);
      },
    };

    await splitDoc({ docs: docs.store, files: renaming, pdfs: pdfs.reader })('kristoff', D);

    expect(docs.rows.get(`kristoff/${D}`)?.doc).toMatchObject({
      title: 'Renamed',
      status: 'ready',
    });
  });
});
