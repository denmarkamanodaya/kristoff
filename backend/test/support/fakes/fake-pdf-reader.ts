// A PdfReader fake: "PDF" bytes are `%PDF-<pages>`; an extracted range is `%PDF-<n pages>` padded to
// `bytesPerPage` per page, so part sizes are controllable without real PDFs.
import type { PdfReader } from '../../../src/ports/pdf-reader.js';
import { appError } from '../../../src/shared/errors/app-error.js';
import { err, ok } from '../../../src/shared/result/result.js';

/** The fake plus the ranges it was asked for. */
export const fakePdfReader = (bytesPerPage = 1000) => {
  const extracted: [number, number][] = [];
  const reader: PdfReader = {
    open: (bytes) => {
      const pages = Number(/^%PDF-(\d+)/.exec(Buffer.from(bytes).toString('latin1'))?.[1] ?? 'x');
      if (!Number.isInteger(pages) || pages < 1)
        return Promise.resolve(err(appError('UPLOAD_INVALID', 'bad')));
      return Promise.resolve(
        ok({
          pageCount: pages,
          extract: (start: number, end: number) => {
            extracted.push([start, end]);
            return Promise.resolve(ok(new Uint8Array((end - start + 1) * bytesPerPage)));
          },
        }),
      );
    },
  };
  return { reader, extracted };
};

/** A "PDF" with `pages` pages, `size` bytes long. */
export const fakePdf = (pages: number, size: number): Uint8Array => {
  const head = Buffer.from(`%PDF-${String(pages)}`);
  const out = new Uint8Array(Math.max(size, head.length));
  out.set(head);
  return out;
};
