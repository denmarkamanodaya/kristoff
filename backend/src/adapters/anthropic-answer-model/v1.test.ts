import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { Message, MessageStreamEvent } from '@anthropic-ai/sdk/resources/messages/messages';
import type { AnswerEvent, ModelRequest } from '../../domain/chats/model-request.js';
import { SYSTEM_PROMPT } from '../../contracts/model-config.js';
import { anthropicAnswerModel, toParams, type MessagesClient } from './v1.js';

const FIXTURE = JSON.parse(
  readFileSync(
    new URL('../../../../contracts/fixtures/anthropic-request.json', import.meta.url),
    'utf8',
  ),
) as { request: ModelRequest; body: { model: string; max_tokens: number; messages: unknown[] } };

const delta = (d: unknown): MessageStreamEvent =>
  ({ type: 'content_block_delta', index: 0, delta: d }) as MessageStreamEvent;

const FINAL = {
  stop_reason: 'end_turn',
  usage: {
    input_tokens: 11,
    output_tokens: 22,
    cache_read_input_tokens: 33,
    cache_creation_input_tokens: null,
  },
} as unknown as Message;

const stubClient = (events: MessageStreamEvent[], fail?: Error) => {
  const bodies: unknown[] = [];
  const client: MessagesClient = {
    stream: (body) => {
      bodies.push(body);
      return {
        async *[Symbol.asyncIterator]() {
          for (const e of events) yield await Promise.resolve(e);
          if (fail) throw fail;
        },
        finalMessage: () => Promise.resolve(FINAL),
      };
    },
  };
  return { client, bodies };
};

describe('anthropic-answer-model v1', () => {
  it('sends exactly the fixture body: url documents, citations, cache breakpoint on the last doc', () => {
    const { model, max_tokens, messages } = FIXTURE.body;
    const expected = JSON.stringify({ model, max_tokens, system: SYSTEM_PROMPT, messages });

    expect(JSON.stringify(toParams(FIXTURE.request, SYSTEM_PROMPT))).toBe(expected);
  });

  it('passes that body to messages.stream and maps text, page citations and usage', async () => {
    const { client, bodies } = stubClient([
      { type: 'message_start' } as unknown as MessageStreamEvent,
      delta({ type: 'text_delta', text: 'Held: ' }),
      delta({
        type: 'citations_delta',
        citation: {
          type: 'page_location',
          cited_text: 'q',
          document_index: 1,
          document_title: 'Civil Code',
          start_page_number: 4,
          end_page_number: 6,
          file_id: null,
        },
      }),
      delta({ type: 'input_json_delta', partial_json: '' }),
    ]);
    const events: AnswerEvent[] = [];

    const result = await anthropicAnswerModel(client, SYSTEM_PROMPT).stream(FIXTURE.request, (e) =>
      events.push(e),
    );

    expect(bodies).toEqual([toParams(FIXTURE.request, SYSTEM_PROMPT)]);
    expect(events).toEqual([
      { kind: 'text', text: 'Held: ' },
      {
        kind: 'citation',
        source: {
          documentIndex: 1,
          documentTitle: 'Civil Code',
          startPage: 4,
          endPageExclusive: 6,
          citedText: 'q',
        },
      },
    ]);
    expect(result).toEqual({
      ok: true,
      value: {
        usage: { input: 11, output: 22, cacheRead: 33, cacheWrite: 0 },
        stopReason: 'end_turn',
      },
    });
  });

  it('maps a non-page citation with null pages', async () => {
    const { client } = stubClient([
      delta({
        type: 'citations_delta',
        citation: {
          type: 'char_location',
          cited_text: 'c',
          document_index: 0,
          document_title: null,
          start_char_index: 0,
          end_char_index: 1,
          file_id: null,
        },
      }),
      delta({
        type: 'citations_delta',
        citation: {
          type: 'web_search_result_location',
          cited_text: 'w',
          url: 'u',
          title: 't',
          encrypted_index: 'e',
        },
      }),
    ]);
    const events: AnswerEvent[] = [];

    await anthropicAnswerModel(client, SYSTEM_PROMPT).stream(FIXTURE.request, (e) =>
      events.push(e),
    );

    expect(events).toEqual([
      {
        kind: 'citation',
        source: {
          documentIndex: 0,
          documentTitle: null,
          startPage: null,
          endPageExclusive: null,
          citedText: 'c',
        },
      },
      {
        kind: 'citation',
        source: {
          documentIndex: -1,
          documentTitle: null,
          startPage: null,
          endPageExclusive: null,
          citedText: 'w',
        },
      },
    ]);
  });

  it('translates an SDK failure midway into UPSTREAM_UNAVAILABLE, keeping delivered events', async () => {
    const failure = Object.assign(new Error('boom'), { name: 'APIConnectionTimeoutError' });
    const { client } = stubClient([delta({ type: 'text_delta', text: 'partial' })], failure);
    const events: AnswerEvent[] = [];

    const result = await anthropicAnswerModel(client, SYSTEM_PROMPT).stream(FIXTURE.request, (e) =>
      events.push(e),
    );

    expect(events).toHaveLength(1);
    expect(result).toMatchObject({
      ok: false,
      error: {
        code: 'UPSTREAM_UNAVAILABLE',
        details: { 'error.type': 'APIConnectionTimeoutError' },
      },
    });
  });
});
