import { PDFDocument } from 'pdf-lib';
import { describe, expect, it } from 'vitest';
import { pdfLibReader } from './v1.js';

const makePdf = async (pages: number): Promise<Uint8Array> => {
  const doc = await PDFDocument.create();
  for (let i = 0; i < pages; i += 1) doc.addPage([200 + i, 200]);
  return doc.save();
};

describe('pdf-lib-reader v1', () => {
  it('counts the pages of a real PDF and extracts a page range as a new PDF', async () => {
    const opened = await pdfLibReader().open(await makePdf(12));
    if (!opened.ok) throw new Error('expected a readable PDF');

    const part = await opened.value.extract(5, 9);

    expect(opened.value.pageCount).toBe(12);
    if (!part.ok) throw new Error('expected a part');
    const reread = await PDFDocument.load(part.value);
    expect(reread.getPageCount()).toBe(5);
    expect(reread.getPage(0).getWidth()).toBe(204);
  });

  it('answers UPLOAD_INVALID for bytes that are not a PDF', async () => {
    expect(await pdfLibReader().open(Buffer.from('not a pdf'))).toMatchObject({
      ok: false,
      error: { code: 'UPLOAD_INVALID', details: { operation: 'pdf.load' } },
    });
  });

  it('answers UPLOAD_INVALID for a range outside the document', async () => {
    const opened = await pdfLibReader().open(await makePdf(2));
    if (!opened.ok) throw new Error('expected a readable PDF');

    expect(await opened.value.extract(2, 5)).toMatchObject({
      ok: false,
      error: { code: 'UPLOAD_INVALID' },
    });
  });
});
