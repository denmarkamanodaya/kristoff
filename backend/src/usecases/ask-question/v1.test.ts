import { describe, expect, it } from 'vitest';
import type { AnswerEnd, AnswerEvent } from '../../domain/chats/model-request.js';
import type { Citation, FollowUps } from '../../domain/model.js';
import type { FollowUpSuggester } from '../../ports/follow-up-suggester.js';
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
const FOLLOW_USAGE = { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 };
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
      followUps: (q: readonly string[]) => log.push(`followups:${String(q.length)}`),
    },
  };
};

const setup = async (
  opts: {
    end?: Result<AnswerEnd, AppError>;
    flags?: Result<boolean, AppError>;
    onFlagRead?: () => void;
    cap?: number;
    followUps?: FollowUps;
  } = {},
) => {
  const chats = memoryChatStore();
  const docs = memoryDocStore();
  const usage = memoryUsageMeter();
  const model = scriptedModel(EVENTS, opts.end ?? ok({ usage: USAGE, stopReason: 'end_turn' }));
  await docs.store.create('kristoff', aDoc({ id: A }), 'k');
  await docs.store.create('kristoff', aDoc({ id: 'bbbbbbbbbbbbbbbb', status: 'pending' }), 'k');
  await chats.store.createChat('kristoff', aChat({ docIds: [A, 'bbbbbbbbbbbbbbbb'] }));
  const clock = fakeClock();
  const suggestions: unknown[] = [];
  const followUps: FollowUpSuggester = {
    suggest: (input) => {
      suggestions.push(input);
      clock.advance(1500);
      return Promise.resolve(
        opts.followUps ?? { questions: ['What is laches?'], usage: FOLLOW_USAGE },
      );
    },
  };
  const run = askQuestion({
    chats: chats.store,
    docs: docs.store,
    files: memoryFileStore().store,
    model: model.model,
    followUps,
    meter: usage.meter,
    flags: {
      isAiEnabled: () => {
        opts.onFlagRead?.();
        return fixedFlags(opts.flags ?? ok(true)).isAiEnabled();
      },
    },
    ids: fakeIds(),
    clock,
    config: {
      modelIds: ['claude-sonnet-5-5', 'claude-opus-5-5'],
      maxTokens: 4096,
      maxHistoryTurns: 20,
      dailyQuestionCap: opts.cap ?? 3,
    },
  });
  return { chats, usage, model, run, suggestions };
};

describe('ask-question v1', () => {
  it('streams text and numbered citations after open, and stores both messages', async () => {
    const { chats, run } = await setup();
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, { text: 'What is estoppel?' }, sink);

    expect(log).toEqual([
      'open',
      'text:Estoppel bars',
      'citation:1',
      'text: denial.',
      'followups:1',
    ]);
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

    await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink);

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

    await run(ME, aChat().id, { text: '  Is a verbal lease valid? ' }, recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat).toMatchObject({
      title: 'Is a verbal lease valid?',
      updatedAt: '2026-11-12T01:00:00.000Z',
    });
    // The answer's tokens plus the follow-up call's.
    const both = { input: 11, output: 21, cacheRead: 30, cacheWrite: 40 };
    expect(usage.tokens.get('kristoff/2026-11-12')).toEqual(both);
    expect(usage.tokens.get('kristoff/TOTAL')).toEqual(both);
  });

  it('titles a chat from the first 60 characters when it has no messages, even if renamed', async () => {
    const { chats, run } = await setup();
    await chats.store.updateChat('kristoff', aChat().id, { title: 'Renamed', updatedAt: 'x' }, 1);

    await run(ME, aChat().id, { text: 'q'.repeat(80) }, recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat.title).toBe('q'.repeat(60));
  });

  it('keeps the title once the chat has messages', async () => {
    const { chats, run } = await setup();
    await run(ME, aChat().id, { text: 'First question' }, recordingSink().sink);

    await run(ME, aChat().id, { text: 'Second question' }, recordingSink().sink);

    expect(chats.chats.get(`kristoff/${aChat().id}`)?.chat.title).toBe('First question');
  });

  it('calls the model the client chose and stores that id on the answer', async () => {
    const { chats, model, run } = await setup();

    await run(ME, aChat().id, { text: 'Q', model: 'claude-opus-5-5' }, recordingSink().sink);

    expect(model.requests[0]?.model).toBe('claude-opus-5-5');
    expect(chats.messages.get(aChat().id)?.[1]?.model).toBe('claude-opus-5-5');
  });

  it('uses the first allowed model when none is chosen', async () => {
    const { chats, model, run } = await setup();

    await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink);

    expect([model.requests[0]?.model, chats.messages.get(aChat().id)?.[1]?.model]).toEqual([
      'claude-sonnet-5-5',
      'claude-sonnet-5-5',
    ]);
  });

  it('refuses a model outside MODEL_IDS with VALIDATION_FAILED before the kill switch and meter', async () => {
    const flagReads: unknown[] = [];
    const { chats, usage, model, run } = await setup({
      flags: ok(true),
      onFlagRead: () => flagReads.push(1),
    });
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, { text: 'Q', model: 'claude-unknown-9' }, sink);

    expect(result).toMatchObject({ ok: false, error: { code: 'VALIDATION_FAILED' } });
    expect([flagReads, usage.questions.size, model.requests.length, log]).toEqual([[], 0, 0, []]);
    expect(chats.messages.get(aChat().id)).toBeUndefined();
  });

  it('suggests follow-ups from the question and the first 2,000 answer chars, stores them with durationMs', async () => {
    const { chats, suggestions, run } = await setup();

    await run(ME, aChat().id, { text: 'What is estoppel?' }, recordingSink().sink);

    expect(suggestions).toEqual([
      { question: 'What is estoppel?', answerExcerpt: 'Estoppel bars [1] denial.' },
    ]);
    expect(chats.messages.get(aChat().id)?.[1]).toMatchObject({
      followUps: ['What is laches?'],
      durationMs: 1500,
    });
    expect(chats.messages.get(aChat().id)?.[0]).toMatchObject({ followUps: [], durationMs: null });
  });

  it('adds the follow-up tokens to usage without counting another question', async () => {
    const { usage, run } = await setup({ cap: 1 });

    const result = await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink);

    expect(result).toMatchObject({ ok: true, value: { usage: USAGE } });
    expect(usage.tokens.get('kristoff/TOTAL')).toEqual({
      input: 11,
      output: 21,
      cacheRead: 30,
      cacheWrite: 40,
    });
    expect(usage.questions.get('kristoff/2026-11-12')).toBe(1);
  });

  it('stores [] and still succeeds when the suggester finds nothing', async () => {
    const { chats, run } = await setup({ followUps: { questions: [], usage: null } });
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, { text: 'Q' }, sink);

    expect(result.ok).toBe(true);
    expect(log.at(-1)).toBe('followups:0');
    expect(chats.messages.get(aChat().id)?.[1]?.followUps).toEqual([]);
  });

  it('does not ask for follow-ups after a failed answer', async () => {
    const { suggestions, run } = await setup({
      end: err(appError('UPSTREAM_UNAVAILABLE', 'down')),
    });
    const { sink, log } = recordingSink();

    await run(ME, aChat().id, { text: 'Q' }, sink);

    expect([suggestions, log.some((l) => l.startsWith('followups'))]).toEqual([[], false]);
  });

  it('refuses before the meter and before open when the kill switch is off', async () => {
    const { usage, run } = await setup({ flags: ok(false) });
    const { sink, log } = recordingSink();

    const result = await run(ME, aChat().id, { text: 'Q' }, sink);

    expect(result).toMatchObject({ error: { code: 'AI_DISABLED' } });
    expect([log, usage.questions.size]).toEqual([[], 0]);
  });

  it('fails closed when the kill switch cannot be read', async () => {
    const { run } = await setup({ flags: err(appError('UPSTREAM_UNAVAILABLE', 'ssm')) });

    expect(await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink)).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });

  it('refuses the question over the daily cap with Retry-After to the next UTC day (clock moved 1.5 s by the first answer)', async () => {
    const { model, run } = await setup({ cap: 1 });
    await run(ME, aChat().id, { text: 'Q1' }, recordingSink().sink);
    const { sink, log } = recordingSink();

    const second = await run(ME, aChat().id, { text: 'Q2' }, sink);

    expect(second).toMatchObject({
      error: { code: 'DAILY_CAP_REACHED', details: { retryAfterSeconds: 23 * 3600 - 1 } },
    });
    expect([log, model.requests.length]).toEqual([[], 1]);
  });

  it('answers CHAT_NOT_FOUND before metering', async () => {
    const { usage, run } = await setup();

    expect(await run(ME, 'ffffffffffffffff', { text: 'Q' }, recordingSink().sink)).toMatchObject({
      error: { code: 'CHAT_NOT_FOUND' },
    });
    expect(usage.questions.size).toBe(0);
  });

  it('stores the partial answer with a note and stopReason "error" when the model fails midway', async () => {
    const { chats, usage, run } = await setup({
      end: err(appError('UPSTREAM_UNAVAILABLE', 'down')),
    });

    const result = await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink);

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

    await run(ME, aChat().id, { text: 'Q' }, recordingSink().sink);

    expect(chats.messages.get(aChat().id)?.[1]?.text).toMatch(/Ask "continue" for the rest/);
  });
});
