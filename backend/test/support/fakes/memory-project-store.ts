// In-memory ProjectStore; trusted because test/ports runs the same contract suite.
import type { Project } from '../../../src/domain/model.js';
import type { ProjectStore } from '../../../src/ports/project-store.js';
import { appError } from '../../../src/shared/errors/app-error.js';
import { err, ok } from '../../../src/shared/result/result.js';

/** The fake plus its raw rows. */
export const memoryProjectStore = () => {
  const rows = new Map<string, { project: Project; version: number }>();
  const k = (uid: string, id: string) => `${uid}/${id}`;
  const count = (uid: string) => [...rows.keys()].filter((key) => key.startsWith(`${uid}/`)).length;
  const store: ProjectStore = {
    create: (uid, project, cap) => {
      if (count(uid) >= cap) return Promise.resolve(err(appError('PROJECT_LIMIT_REACHED', 'cap')));
      if (rows.has(k(uid, project.id))) return Promise.resolve(err(appError('INTERNAL', 'exists')));
      rows.set(k(uid, project.id), { project, version: 1 });
      return Promise.resolve(ok(undefined));
    },
    get: (uid, id) => {
      const row = rows.get(k(uid, id));
      return Promise.resolve(
        row
          ? ok({ value: row.project, version: row.version })
          : err(appError('PROJECT_NOT_FOUND', 'no')),
      );
    },
    list: (uid) =>
      Promise.resolve(
        ok(
          [...rows.entries()]
            .filter(([key]) => key.startsWith(`${uid}/`))
            .map(([, r]) => r.project),
        ),
      ),
    rename: (uid, id, change) => {
      const row = rows.get(k(uid, id));
      if (!row) return Promise.resolve(err(appError('PROJECT_NOT_FOUND', 'no')));
      if (row.version !== change.version)
        return Promise.resolve(err(appError('VERSION_CONFLICT', 'stale')));
      const project = { ...row.project, name: change.name, updatedAt: change.updatedAt };
      rows.set(k(uid, id), { project, version: row.version + 1 });
      return Promise.resolve(ok(project));
    },
    delete: (uid, id) =>
      Promise.resolve(
        rows.delete(k(uid, id)) ? ok(undefined) : err(appError('PROJECT_NOT_FOUND', 'no')),
      ),
  };
  return { store, rows };
};
