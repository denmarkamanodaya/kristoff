import { describe, expect, it } from 'vitest';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { memoryProjectStore } from '../../../test/support/fakes/memory-project-store.js';
import { renameProject } from './v1.js';

const P = { id: 'pppppppppppppppp', name: 'Old', createdAt: 'a', updatedAt: 'a' };

describe('rename-project v1', () => {
  it('renames (trimmed) and bumps updatedAt', async () => {
    const projects = memoryProjectStore();
    await projects.store.create('kristoff', P, 50);

    const result = await renameProject({ projects: projects.store, clock: fakeClock() })(
      { uid: 'kristoff' },
      P.id,
      ' New ',
    );

    expect(result).toEqual({
      ok: true,
      value: { ...P, name: 'New', updatedAt: '2026-11-12T01:00:00.000Z' },
    });
  });

  it("refuses a blank name and another owner's project", async () => {
    const projects = memoryProjectStore();
    await projects.store.create('eve', P, 50);
    const run = renameProject({ projects: projects.store, clock: fakeClock() });

    expect(await run({ uid: 'kristoff' }, P.id, ' ')).toMatchObject({
      error: { code: 'VALIDATION_FAILED' },
    });
    expect(await run({ uid: 'kristoff' }, P.id, 'x')).toMatchObject({
      error: { code: 'PROJECT_NOT_FOUND' },
    });
  });
});
