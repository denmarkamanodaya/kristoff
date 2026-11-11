// PdfReader port: page count and page-range extraction of a PDF (docs/contract.md "v2 Large PDFs").
// No library types cross it.
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';

/** A parsed PDF, kept open so several ranges can be cut from one parse. */
export interface OpenPdf {
  /** Pages in the document (≥ 1). */
  readonly pageCount: number;
  /** A new PDF of pages `startPage..endPage` (1-based, inclusive). */
  extract(startPage: number, endPage: number): Promise<Result<Uint8Array, AppError>>;
}

/** Opens PDFs. */
export interface PdfReader {
  /** Parses the bytes; `UPLOAD_INVALID` when they are not a readable PDF. */
  open(bytes: Uint8Array): Promise<Result<OpenPdf, AppError>>;
}
