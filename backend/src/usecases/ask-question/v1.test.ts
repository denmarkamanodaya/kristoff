import { describe, expect, it } from 'vitest';
import type { AnswerEnd, AnswerEvent } from '../../domain/chats/model-request.js';
import type { Citation } from '../../domain/model.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import { aChat, aDoc } from '../../../test/support/builders.js';
import { fakeClock, fakeIds } from '../../../test/support/fakes/clock-ids.js';
import { fixedFlags, scriptedModel } from '../../../test/support/fakes/fixed-ports.js';
import { memoryChatStore } from '../../../test/support/fakes/memory-chat-store.js';
import { memoryDocStore } from '../../../test/support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../../../test/support/fakes/memory-file-store.js';
import { memoryUsageMeter } from '../../../test/support/fakes/memory-usage-meter.js';
import { askQuestion } from './v1.js';

const A = 'aaaaaaaaaaaaaaaa';
const ME = { uid: 'kristoff' };
const USAGE = { input: 10, output: 20, cacheRead: 30, cacheWrite: 40 };
const EVENTS: AnswerEvent[] = [
  { kind: 'text', text: 'Estoppel bars' },
  {
    kind: 'citation',
    source: {
      documentIndex: 0,
      documentTitle: null,
      startPage: 2,
      endPageExclusive: 3,
      citedText: 'q',
    },
  },
  { kind: 'text', text: ' denial.' },
];

const recordingSink = () => {
  const log: string[] = [];
  return {
    log,
    sink: {
      open: () => log.push('open'),
      text: (t: string) => log.push(`text:${t}`),
      citation: (c: Citation) => log.push(`citation:${String(c.n)}`),
    },
  };
};

const setup = async (
  opts: {
    end?: Result<AnswerEnd, AppError>;
    flags?: Result<boolean, AppError>;
    cap?: number;
  } = {},
) => {
  const chats = memoryChatStore();
  const docs = memoryDocStore();
  const usage = memoryUsageMeter();
  const model = scriptedModel(EVENTS, opts.end ?? ok({ usage: USAGE, stopReason: 'end_turn' }));
  await docs.store.create('kristoff', aDoc({ id: A }), 'k');
  await docs.store.create('kristoff', aDoc({ id: 'bbbbbbbbbbbbbbbb', status: 'pending' }), 'k');
  await chats.store.createChat('kristoff', aChat({ docIds: [A, 'bbbbbbbbbbbbbbbb'] }));
  const run = askQuestion({
    chats: chats.store,
    docs: docs.store,
    files: memoryFileStore().store,
    model: model.model,
    meter: usage.meter,
    flags: fixedFlags(opts.flags ?? ok(true)),
    ids: fakeIds(),
    clock: fakeClock(),
    config: {
      modelId: 'claude-sonnet-5-5',
      maxTokens: 4096,
      maxHistoryTurns: 20,
      dailyQuestionCap: opts.cap ?? 3,
    },
  });
  return { chats, usage, model, run };
};

describe('ask-question v1', () => {
  it('streams text and numbered citations after open, and stores both messages', async () => {
    const { chats, run } = await setup();
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, 'What is estoppel?', sink);

    expect(log).toEqual(['open', 'text:Estoppel bars', 'citation:1', 'text: denial.']);
    expect(result).toMatchObject({ ok: true, value: { usage: USAGE } });
    expect(chats.messages.get(aChat().id)).toEqual([
      expect.objectContaining({ role: 'user', text: 'What is estoppel?', usage: null }),
      expect.objectContaining({
        role: 'assistant',
        text: 'Estoppel bars [1] denial.',
        citations: [
          { n: 1, docId: A, docTitle: 'People v. Cruz', page: 2, endPage: 2, quote: 'q' },
        ],
        usage: USAGE,
        model: 'claude-sonnet-5-5',
        stopReason: 'end_turn',
      }),
    ]);
  });

  it('sends only ready docs, by presigned URL, to the configured model and max tokens', async () => {
    const { model, run } = await setup();

    await run(ME, aChat().id, 'Q', recordingSink().sink);

    expect(model.requests[0]).toEqual({
      model: 'claude-sonnet-5-5',
      maxTokens: 4096,
      turns: [
        {
          role: 'user',
          blocks: [
            {
              kind: 'doc',
              title: 'People v. Cruz',
              url: `https://get.example/docs/kristoff/${A}.pdf`,
              isCacheBreakpoint: true,
            },
            { kind: 'text', text: 'Q' },
          ],
        },
      ],
    });
  });

  it('titles a new chat from the first question and counts tokens for the day and total', async () => {
    const { chats, usage, run } = await setup();

    await run(ME, aChat().id, '  Is a verbal lease valid? ', recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat).toMatchObject({
      title: 'Is a verbal lease valid?',
      updatedAt: '2026-11-12T01:00:00.000Z',
    });
    expect(usage.tokens.get('kristoff/2026-11-12')).toEqual(USAGE);
    expect(usage.tokens.get('kristoff/TOTAL')).toEqual(USAGE);
  });

  it('titles a chat from the first 60 characters when it has no messages, even if renamed', async () => {
    const { chats, run } = await setup();
    await chats.store.updateChat('kristoff', aChat().id, { title: 'Renamed', updatedAt: 'x' }, 1);

    await run(ME, aChat().id, 'q'.repeat(80), recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat.title).toBe('q'.repeat(60));
  });

  it('keeps the title once the chat has messages', async () => {
    const { chats, run } = await setup();
    await run(ME, aChat().id, 'First question', recordingSink().sink);

    await run(ME, aChat().id, 'Second question', recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat.title).toBe('First question');
  });

  it('refuses before the meter and before open when the kill switch is off', async () => {
    const { usage, run } = await setup({ flags: ok(false) });
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, 'Q', sink);

    expect(result).toMatchObject({ error: { code: 'AI_DISABLED' } });
    expect([log, usage.questions.size]).toEqual([[], 0]);
  });

  it('fails closed when the kill switch cannot be read', async () => {
    const { run } = await setup({ flags: err(appError('UPSTREAM_UNAVAILABLE', 'ssm')) });

    expect(await run(ME, aChat().id, 'Q', recordingSink().sink)).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });

  it('refuses the question over the daily cap with Retry-After to the next UTC day', async () => {
    const { model, run } = await setup({ cap: 1 });
    await run(ME, aChat().id, 'Q1', recordingSink().sink);
    const { sink, log } = recordingSink();

    const second = await run(ME, aChat().id, 'Q2', sink);

    expect(second).toMatchObject({
      error: { code: 'DAILY_CAP_REACHED', details: { retryAfterSeconds: 23 * 3600 } },
    });
    expect([log, model.requests.length]).toEqual([[], 1]);
  });

  it('answers CHAT_NOT_FOUND before metering', async () => {
    const { usage, run } = await setup();

    expect(await run(ME, 'ffffffffffffffff', 'Q', recordingSink().sink)).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
    expect(usage.questions.size).toBe(0);
  });

  it('stores the partial answer with a note and stopReason "error" when the model fails midway', async () => {
    const { chats, usage, run } = await setup({
      end: err(appError('UPSTREAM_UNAVAILABLE', 'down')),
    });

    const result = await run(ME, aChat().id, 'Q', recordingSink().sink);

    expect(result).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(chats.messages.get(aChat().id)?.[1]).toMatchObject({
      role: 'assistant',
      text: 'Estoppel bars [1] denial.\n\n_(The answer was interrupted. Please ask again.)_',
      stopReason: 'error',
      usage: null,
    });
    expect(usage.tokens.size).toBe(0);
  });

  it('notes a cut-off answer when the model stops at max_tokens', async () => {
    const { chats, run } = await setup({ end: ok({ usage: USAGE, stopReason: 'max_tokens' }) });

    await run(ME, aChat().id, 'Q', recordingSink().sink);

    expect(chats.messages.get(aChat().id)?.[1]?.text).toMatch(/Ask "continue" for the rest/);
  });
});
