// PdfLibReader: the PdfReader port on pdf-lib 1.17.1 (docs/contract.md "v2 Large PDFs"). Parses
// once, then copies page ranges into new PDFs. An unreadable or encrypted PDF is UPLOAD_INVALID;
// pdf-lib exceptions never cross the port. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { PDFDocument } from 'pdf-lib';
import type { OpenPdf, PdfReader } from '../../ports/pdf-reader.js';
import { appError } from '../../shared/errors/app-error.js';
import { errorTypeOf } from '../../shared/errors/vendor-error.js';
import { err, ok } from '../../shared/result/result.js';

const opened = (source: PDFDocument): OpenPdf => ({
  pageCount: source.getPageCount(),
  extract: async (startPage, endPage) => {
    try {
      const part = await PDFDocument.create({ updateMetadata: false });
      const indices = Array.from({ length: endPage - startPage + 1 }, (_, i) => startPage - 1 + i);
      const pages = await part.copyPages(source, indices);
      pages.forEach((page) => part.addPage(page));
      return ok(await part.save({ useObjectStreams: true }));
    } catch (error) {
      return err(
        appError('UPLOAD_INVALID', 'pages could not be copied', {
          operation: 'pdf.extract',
          'error.type': errorTypeOf(error),
        }),
      );
    }
  },
});

/**
 * Builds the pdf-lib PDF reader.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @returns The PdfReader.
 */
export const pdfLibReader = (): PdfReader => ({
  open: async (bytes) => {
    try {
      const source = await PDFDocument.load(bytes, { updateMetadata: false });
      return source.getPageCount() > 0
        ? ok(opened(source))
        : err(appError('UPLOAD_INVALID', 'the PDF has no pages'));
    } catch (error) {
      return err(
        appError('UPLOAD_INVALID', 'the PDF could not be read', {
          operation: 'pdf.load',
          'error.type': errorTypeOf(error),
        }),
      );
    }
  },
});
