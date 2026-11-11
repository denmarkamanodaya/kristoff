import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import type { AppError } from '../../shared/errors/app-error.js';
import { aChat } from '../../../test/support/builders.js';
import { fakeClock } from '../../../test/support/fakes/clock-ids.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { updateChat } from './v1.js';

type Resolve = (p: unknown, ids: readonly string[]) => Promise<Result<string[], AppError>>;
const echo: Resolve = (_p, ids) => Promise.resolve(ok([...ids]));

const setup = async (resolveDocs: Resolve = echo) => {
  const chats = memoryChatStore();
  await chats.store.createChat('kristoff', aChat());
  return { chats, run: updateChat({ chats: chats.store, clock: fakeClock(), resolveDocs }) };
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
});
