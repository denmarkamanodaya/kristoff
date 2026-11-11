import { describe, expect, it } from 'vitest';
import { fakeClock, fakeIds } from '../../../test/support/fakes/clock-ids.js';
import { memoryProjectStore } from '../../../test/support/fakes/memory-project-store.js';
import { createProject } from './v1.js';

describe('create-project v1', () => {
  it('creates a project with a trimmed name at the clock instant', async () => {
    const projects = memoryProjectStore();
    const run = createProject({ projects: projects.store, ids: fakeIds(), clock: fakeClock() });

    expect(await run({ uid: 'kristoff' }, '  Torts ')).toEqual({
      ok: true,
      value: {
        id: '0000000000000001',
        name: 'Torts',
        createdAt: '2026-11-12T01:00:00.000Z',
        updatedAt: '2026-11-12T01:00:00.000Z',
      },
    });
  });

  it('refuses the 51st project with PROJECT_LIMIT_REACHED and a blank name with VALIDATION_FAILED', async () => {
    const projects = memoryProjectStore();
    const run = createProject({ projects: projects.store, ids: fakeIds(), clock: fakeClock() });
    for (let i = 0; i < 50; i += 1) await run({ uid: 'kristoff' }, `P${String(i)}`);

    expect(await run({ uid: 'kristoff' }, 'P50')).toMatchObject({
      error: { code: 'PROJECT_LIMIT_REACHED' },
    });
    expect(await run({ uid: 'eve' }, ' ')).toMatchObject({ error: { code: 'VALIDATION_FAILED' } });
    expect((await run({ uid: 'eve' }, 'First')).ok).toBe(true);
  });
});
