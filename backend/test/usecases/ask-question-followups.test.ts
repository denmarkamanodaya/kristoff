// ask-question v1: the follow-up excerpt rule (split out of the unit test file for its size cap).
import { describe, expect, it } from 'vitest';
import type { AnswerEvent } from '../../src/domain/chats/model-request.js';
import type { AnswerSink } from '../../src/ports/answer-sink.js';
import { ok } from '../../src/shared/result/result.js';
import { askQuestion } from '../../src/usecases/ask-question/index.js';
import { aChat } from '../support/builders.js';
import { fakeClock, fakeIds } from '../support/fakes/clock-ids.js';
import { fixedFlags, scriptedModel } from '../support/fakes/fixed-ports.js';
import { memoryChatStore } from '../support/fakes/memory-chat-store.js';
import { memoryDocStore } from '../support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../support/fakes/memory-file-store.js';
import { memoryUsageMeter } from '../support/fakes/memory-usage-meter.js';

const ME = { uid: 'kristoff' };
const USAGE = { input: 10, output: 20, cacheRead: 30, cacheWrite: 40 };
const sink: AnswerSink = {
  open: () => undefined,
  text: () => undefined,
  citation: () => undefined,
  followUps: () => undefined,
};

describe('ask-question v1 follow-ups', () => {
  it('passes only the first 2,000 characters of a long answer to the suggester', async () => {
    const chats = memoryChatStore();
    const docs = memoryDocStore();
    await chats.store.createChat('kristoff', aChat());
    const long: AnswerEvent[] = [{ kind: 'text', text: 'x'.repeat(2500) }];
    const seen: { answerExcerpt: string }[] = [];
    const run = askQuestion({
      chats: chats.store,
      docs: docs.store,
      files: memoryFileStore().store,
      model: scriptedModel(long, ok({ usage: USAGE, stopReason: 'end_turn' })).model,
      followUps: {
        suggest: (i) => (seen.push(i), Promise.resolve({ questions: [], usage: null })),
      },
      meter: memoryUsageMeter().meter,
      flags: fixedFlags(ok(true)),
      ids: fakeIds(),
      clock: fakeClock(),
      config: { modelIds: ['m'], maxTokens: 1, maxHistoryTurns: 0, dailyQuestionCap: 1 },
    });

    await run(ME, aChat().id, { text: 'Q' }, sink);

    expect(seen[0]?.answerExcerpt).toHaveLength(2000);
  });
});
