// In-memory DocStore; trusted because test/ports runs the same contract suite as the real adapter.
import type { Doc, Page, Versioned } from '../../../src/domain/model.js';
import type { DocPatch, DocStore } from '../../../src/ports/doc-store.js';
import { appError, type AppError } from '../../../src/shared/errors/app-error.js';
import { err, ok, type Result } from '../../../src/shared/result/result.js';

/** The fake plus its raw rows for assertions. */
export const memoryDocStore = () => {
  const rows = new Map<string, { doc: Doc; version: number; objectKey: string }>();
  const k = (uid: string, id: string) => `${uid}/${id}`;
  const store: DocStore = {
    create: (uid, doc, objectKey) => {
      if (rows.has(k(uid, doc.id))) return Promise.resolve(err(appError('INTERNAL', 'exists')));
      rows.set(k(uid, doc.id), { doc, version: 1, objectKey });
      return Promise.resolve(ok(undefined));
    },
    get: (uid, id): Promise<Result<Versioned<Doc>, AppError>> => {
      const row = rows.get(k(uid, id));
      return Promise.resolve(
        row ? ok({ value: row.doc, version: row.version }) : err(appError('DOC_NOT_FOUND', 'no')),
      );
    },
    getMany: (uid, ids) =>
      Promise.resolve(
        ok(
          ids.flatMap((id) => {
            const row = rows.get(k(uid, id));
            return row ? [row.doc] : [];
          }),
        ),
      ),
    list: (uid, cursor, limit): Promise<Result<Page<Doc>, AppError>> => {
      if (cursor !== undefined && !/^\d+$/.test(cursor))
        return Promise.resolve(err(appError('VALIDATION_FAILED', 'bad cursor')));
      const all = [...rows.entries()]
        .filter(([key]) => key.startsWith(`${uid}/`))
        .map(([, row]) => row.doc)
        .sort((a, b) => b.id.localeCompare(a.id));
      const start = cursor === undefined ? 0 : Number(cursor);
      const items = all.slice(start, start + limit);
      const next = start + limit < all.length ? String(start + limit) : null;
      return Promise.resolve(ok({ items, nextCursor: next }));
    },
    update: (uid, id, patch: DocPatch, version) => {
      const row = rows.get(k(uid, id));
      if (!row) return Promise.resolve(err(appError('DOC_NOT_FOUND', 'no')));
      if (row.version !== version)
        return Promise.resolve(err(appError('VERSION_CONFLICT', 'stale')));
      const doc = { ...row.doc, ...patch };
      rows.set(k(uid, id), { ...row, doc, version: version + 1 });
      return Promise.resolve(ok(doc));
    },
  };
  return { store, rows };
};
