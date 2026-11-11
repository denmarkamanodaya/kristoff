// ask-question v1 with a split document: each attached part is its own document block titled with
// its page range, and citation pages are mapped back to original pages (docs/contract.md "v2").
import { describe, expect, it } from 'vitest';
import type { AnswerEvent } from '../../src/domain/chats/model-request.js';
import type { AnswerSink } from '../../src/ports/answer-sink.js';
import { ok } from '../../src/shared/result/result.js';
import { askQuestion } from '../../src/usecases/ask-question/index.js';
import { aChat, aDoc } from '../support/builders.js';
import { fakeClock, fakeIds } from '../support/fakes/clock-ids.js';
import { fixedFlags, scriptedModel } from '../support/fakes/fixed-ports.js';
import { memoryChatStore } from '../support/fakes/memory-chat-store.js';
import { memoryDocStore } from '../support/fakes/memory-doc-store.js';
import { memoryFileStore } from '../support/fakes/memory-file-store.js';
import { memoryUsageMeter } from '../support/fakes/memory-usage-meter.js';

const D = 'aaaaaaaaaaaaaaaa';
const sink: AnswerSink = {
  open: () => undefined,
  text: () => undefined,
  citation: () => undefined,
  followUps: () => undefined,
};

describe('ask-question v1 with parts', () => {
  it('sends parts as titled documents and maps cited pages back to the original document', async () => {
    const chats = memoryChatStore();
    const docs = memoryDocStore();
    const parts = [
      { n: 1, startPage: 1, endPage: 100, size: 9 },
      { n: 2, startPage: 101, endPage: 200, size: 9 },
    ];
    await docs.store.create(
      'kristoff',
      aDoc({ id: D, title: 'Civil Code', parts, pageCount: 200 }),
      'k',
    );
    await chats.store.createChat('kristoff', aChat({ docIds: [`${D}:p2`, D] }));
    const events: AnswerEvent[] = [
      {
        kind: 'citation',
        source: {
          documentIndex: 0,
          documentTitle: 'Civil Code (pp. 101–200)',
          startPage: 5,
          endPageExclusive: 7,
          citedText: 'q',
        },
      },
    ];
    const model = scriptedModel(
      events,
      ok({ usage: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0 }, stopReason: 'end_turn' }),
    );
    const run = askQuestion({
      chats: chats.store,
      docs: docs.store,
      files: memoryFileStore().store,
      model: model.model,
      followUps: { suggest: () => Promise.resolve({ questions: [], usage: null }) },
      meter: memoryUsageMeter().meter,
      flags: fixedFlags(ok(true)),
      ids: fakeIds(),
      clock: fakeClock(),
      config: { modelIds: ['m'], maxTokens: 1, maxHistoryTurns: 0, dailyQuestionCap: 1 },
    });

    await run({ uid: 'kristoff' }, aChat().id, { text: 'Q' }, sink);

    expect(model.requests[0]?.turns[0]?.blocks.slice(0, -1)).toEqual([
      {
        kind: 'doc',
        title: 'Civil Code (pp. 101–200)',
        url: `https://get.example/docs/kristoff/${D}/part-2.pdf`,
        isCacheBreakpoint: true,
      },
    ]);
    expect(chats.messages.get(aChat().id)?.[1]?.citations).toEqual([
      { n: 1, docId: D, docTitle: 'Civil Code', page: 105, endPage: 106, quote: 'q' },
    ]);
  });
});
