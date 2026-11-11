import { describe, expect, it } from 'vitest';
import { memoryProjectStore } from '../../../test/support/fakes/memory-project-store.js';
import { listProjects } from './v1.js';

const at = '2026-11-12T00:00:00.000Z';

describe('list-projects v1', () => {
  it("lists only the principal's projects in name order, tie-break on id", async () => {
    const projects = memoryProjectStore();
    for (const [id, name] of [
      ['3', 'Torts'],
      ['1', 'Civil'],
      ['2', 'Civil'],
      ['4', 'Admin'],
    ])
      await projects.store.create(
        'kristoff',
        { id: String(id), name: String(name), createdAt: at, updatedAt: at },
        50,
      );
    await projects.store.create('eve', { id: '9', name: 'Aaa', createdAt: at, updatedAt: at }, 50);

    const result = await listProjects({ projects: projects.store })({ uid: 'kristoff' });

    expect(result.ok && result.value.map((p) => p.id)).toEqual(['4', '1', '2', '3']);
  });
});
