import { describe, expect, it } from 'vitest';
import type { ChatStore } from '../../ports/chat-store.js';
import type { ProjectStore } from '../../ports/project-store.js';
import { appError } from '../../shared/errors/app-error.js';
import { err } from '../../shared/result/result.js';
import { aChat } from '../../../test/support/builders.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { memoryProjectStore } from '../../../test/support/fakes/memory-project-store.js';
import { deleteProject } from './v1.js';

const PID = 'pppppppppppppppp';

const setup = async () => {
  const chats = memoryChatStore();
  const projects = memoryProjectStore();
  await projects.store.create(
    'kristoff',
    { id: PID, name: 'T', createdAt: 'a', updatedAt: 'a' },
    50,
  );
  for (let i = 0; i < 60; i += 1)
    await chats.store.createChat(
      'kristoff',
      aChat({
        id: i.toString(16).padStart(16, '0'),
        projectId: i % 2 === 0 ? PID : null,
        updatedAt: new Date(Date.UTC(2026, 0, 1) + i).toISOString(),
      }),
    );
  return { chats, projects };
};

describe('delete-project v1', () => {
  it('ungroups every chat of the project across pages, then deletes the project (in that order)', async () => {
    const { chats, projects } = await setup();
    const calls: string[] = [];
    const chatStore: ChatStore = {
      ...chats.store,
      clearProject: (u, c, p) => (calls.push('clear'), chats.store.clearProject(u, c, p)),
    };
    const projectStore: ProjectStore = {
      ...projects.store,
      delete: (u, p) => (calls.push('delete'), projects.store.delete(u, p)),
    };

    const result = await deleteProject({ projects: projectStore, chats: chatStore })(
      { uid: 'kristoff' },
      PID,
    );

    expect(result).toEqual({ ok: true, value: undefined });
    expect([calls.filter((c) => c === 'clear').length, calls.at(-1)]).toEqual([30, 'delete']);
    expect([...chats.chats.values()].filter((r) => r.chat.projectId !== null)).toEqual([]);
    expect(projects.rows.size).toBe(0);
  });

  it('keeps the project when ungrouping fails, and answers PROJECT_NOT_FOUND for another owner', async () => {
    const { chats, projects } = await setup();
    const failing: ChatStore = {
      ...chats.store,
      clearProject: () => Promise.resolve(err(appError('UPSTREAM_UNAVAILABLE', 'x'))),
    };

    expect(
      await deleteProject({ projects: projects.store, chats: failing })({ uid: 'kristoff' }, PID),
    ).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(projects.rows.size).toBe(1);
    expect(
      await deleteProject({ projects: projects.store, chats: chats.store })({ uid: 'eve' }, PID),
    ).toMatchObject({ error: { code: 'PROJECT_NOT_FOUND' } });
  });
});
