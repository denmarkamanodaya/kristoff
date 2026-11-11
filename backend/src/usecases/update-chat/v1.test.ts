import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { aChat } from '../../../test/support/builders.js';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { memoryProjectStore } from '../../../test/support/fakes/memory-project-store.js';
import { updateChat } from './v1.js';

type Resolve = (p: unknown, ids: readonly string[]) => Promise<Result<string[], AppError>>;
const echo: Resolve = (_p, ids) => Promise.resolve(ok([...ids]));

const setup = async (resolveDocs: Resolve = echo) => {
  const chats = memoryChatStore();
  const projects = memoryProjectStore();
  await chats.store.createChat('kristoff', aChat());
  const p = { id: 'pppppppppppppppp', name: 'Torts', createdAt: 'x', updatedAt: 'x' };
  await projects.store.create('kristoff', p, 50);
  await projects.store.create('eve', { ...p, id: 'eeeeeeeeeeeeeeee' }, 50);
  const run = updateChat({
    chats: chats.store,
    projects: projects.store,
    clock: fakeClock(),
    resolveDocs,
  });
  return { chats, run };
};

describe('update-chat v1', () => {
  it('renames (trimmed) and bumps updatedAt, keeping the docs', async () => {
    const { run } = await setup();

    const result = await run({ uid: 'kristoff' }, aChat().id, { title: '  Estoppel ' });

    expect(result).toMatchObject({
      ok: true,
      value: { title: 'Estoppel', docIds: [], updatedAt: '2026-11-12T01:00:00.000Z' },
    });
  });

  it('re-attaches the resolved doc ids', async () => {
    const { run } = await setup();

    const result = await run({ uid: 'kristoff' }, aChat().id, { docIds: ['aaaaaaaaaaaaaaaa'] });

    expect(result).toMatchObject({
      ok: true,
      value: { docIds: ['aaaaaaaaaaaaaaaa'], title: 'New chat' },
    });
  });

  it('refuses a blank title, a failed docs check and an unknown chat', async () => {
    const { run } = await setup(() => Promise.resolve(err(appError('DOC_NOT_FOUND', 'x'))));

    expect(await run({ uid: 'kristoff' }, aChat().id, { title: ' ' })).toMatchObject({
      error: { code: 'VALIDATION_FAILED' },
    });
    expect(await run({ uid: 'kristoff' }, aChat().id, { docIds: ['x'] })).toMatchObject({
      error: { code: 'DOC_NOT_FOUND' },
    });
    expect(await run({ uid: 'kristoff' }, 'ffffffffffffffff', { title: 'x' })).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
  });

  it('pins and moves the chat into an existing project of the owner, then ungroups with null', async () => {
    const { run } = await setup();

    const moved = await run({ uid: 'kristoff' }, aChat().id, {
      pinned: true,
      projectId: 'pppppppppppppppp',
    });
    const ungrouped = await run({ uid: 'kristoff' }, aChat().id, { projectId: null });

    expect(moved).toMatchObject({
      ok: true,
      value: { pinned: true, projectId: 'pppppppppppppppp' },
    });
    expect(ungrouped).toMatchObject({ ok: true, value: { pinned: true, projectId: null } });
  });

  it("answers PROJECT_NOT_FOUND for an unknown project and for another owner's project", async () => {
    const { chats, run } = await setup();

    expect(
      await run({ uid: 'kristoff' }, aChat().id, { projectId: 'ffffffffffffffff' }),
    ).toMatchObject({
      error: { code: 'PROJECT_NOT_FOUND' },
    });
    expect(
      await run({ uid: 'kristoff' }, aChat().id, { projectId: 'eeeeeeeeeeeeeeee' }),
    ).toMatchObject({
      error: { code: 'PROJECT_NOT_FOUND' },
    });
    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat.projectId).toBeNull();
  });
});
