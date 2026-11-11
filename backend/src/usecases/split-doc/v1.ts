// docs-split use case (docs/contract.md "v2 Large PDFs"): for a `processing` doc, read the PDF,
// count its pages and either mark it `ready` whole (≤ 100 pages, ≤ 30 MiB) or write parts of
// ≤ 100 pages and ≤ 30 MiB to `docs/<uid>/<docId>/part-<n>.pdf` and mark it `ready` with them.
// Any failure marks it `failed`. Delivery is at least once: a doc that is not `processing` is left
// alone, and parts are overwritten by key. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import {
  halveRange,
  initialRanges,
  needsParts,
  partFits,
  type PageRange,
} from '../../domain/docs/plan-parts/index.js';
import { docObjectKey, docPartKey } from '../../domain/keys/index.js';
import { MAX_PARTS } from '../../domain/limits.js';
import type { DocPart } from '../../domain/model.js';
import type { DocPatch, DocStore } from '../../ports/doc-store.js';
import type { FileStore } from '../../ports/file-store.js';
import type { OpenPdf, PdfReader } from '../../ports/pdf-reader.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';

/** What split-doc needs. */
export interface SplitDocDeps {
  readonly docs: DocStore;
  readonly files: FileStore;
  readonly pdfs: PdfReader;
}

/** How a split ended: the doc was processed, or skipped because it was not `processing`. */
export type SplitOutcome = 'ready' | 'skipped';

/** Cuts ranges, halving any whose output is over 30 MiB, and writes each part in page order. */
const writeParts = async (
  deps: SplitDocDeps,
  target: { readonly uid: string; readonly docId: string; readonly pdf: OpenPdf },
): Promise<Result<DocPart[], AppError>> => {
  const queue: PageRange[] = initialRanges(target.pdf.pageCount);
  const parts: DocPart[] = [];
  for (let range = queue.shift(); range !== undefined; range = queue.shift()) {
    const bytes = await target.pdf.extract(range.startPage, range.endPage);
    if (!bytes.ok) return bytes;
    if (!partFits(bytes.value.length)) {
      const halves = halveRange(range);
      if (halves === undefined)
        return err(appError('UPLOAD_INVALID', 'a single page is over 30 MiB'));
      queue.unshift(...halves);
      continue;
    }
    if (parts.length >= MAX_PARTS) return err(appError('UPLOAD_INVALID', 'too many parts'));
    const n = parts.length + 1;
    const written = await deps.files.put(docPartKey(target.uid, target.docId, n), bytes.value);
    if (!written.ok) return written;
    parts.push({ n, startPage: range.startPage, endPage: range.endPage, size: bytes.value.length });
  }
  return ok(parts);
};

/** The finished state: whole doc, or parts. */
const processPdf = async (
  deps: SplitDocDeps,
  uid: string,
  docId: string,
): Promise<Result<DocPatch, AppError>> => {
  const bytes = await deps.files.read(docObjectKey(uid, docId));
  if (!bytes.ok) return bytes;
  const pdf = await deps.pdfs.open(bytes.value);
  if (!pdf.ok) return pdf;
  const { pageCount } = pdf.value;
  if (!needsParts(pageCount, bytes.value.length))
    return ok({ status: 'ready', pageCount, parts: [] });
  const parts = await writeParts(deps, { uid, docId, pdf: pdf.value });
  return parts.ok ? ok({ status: 'ready', pageCount, parts: parts.value }) : parts;
};

/** Writes the end state at the doc's current version (read just before, so a rename meanwhile is kept). */
const finish = async (
  deps: SplitDocDeps,
  ids: { readonly uid: string; readonly docId: string },
  patch: DocPatch,
): Promise<Result<void, AppError>> => {
  const current = await deps.docs.get(ids.uid, ids.docId);
  if (!current.ok) return current;
  const saved = await deps.docs.update(ids.uid, ids.docId, patch, current.value.version);
  return saved.ok ? ok(undefined) : saved;
};

/**
 * Builds the split-doc use case.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Doc store, file store and PDF reader.
 * @returns A function from the owner id and doc id to `ready` or `skipped`; on a processing
 *   failure the doc is marked `failed` and the failure is returned (`UPLOAD_INVALID` for an
 *   unreadable PDF or an unsplittable page, else a store failure).
 */
export const splitDoc =
  (deps: SplitDocDeps) =>
  async (uid: string, docId: string): Promise<Result<SplitOutcome, AppError>> => {
    const found = await deps.docs.get(uid, docId);
    if (!found.ok) return found.error.code === 'DOC_NOT_FOUND' ? ok('skipped') : found;
    if (found.value.value.status !== 'processing') return ok('skipped');
    const patch = await processPdf(deps, uid, docId);
    if (patch.ok) {
      const saved = await finish(deps, { uid, docId }, patch.value);
      return saved.ok ? ok('ready') : saved;
    }
    const failed = await finish(deps, { uid, docId }, { status: 'failed' });
    return failed.ok ? patch : failed;
  };
