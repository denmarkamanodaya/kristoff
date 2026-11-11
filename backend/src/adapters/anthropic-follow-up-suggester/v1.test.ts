import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { Message } from '@anthropic-ai/sdk/resources/messages/messages';
import {
  FOLLOW_UP_MAX_TOKENS,
  FOLLOW_UP_PROMPT,
  FOLLOW_UP_SCHEMA,
  FOLLOW_UP_TIMEOUT_MS,
} from '../../contracts/model-config.js';
import { httpDeps } from '../../../test/support/http-events.js';
import { anthropicFollowUpSuggester, toFollowUpParams, type CreateClient } from './v1.js';

const FIXTURE = JSON.parse(
  readFileSync(
    new URL('../../../../contracts/fixtures/anthropic-followups-request.json', import.meta.url),
    'utf8',
  ),
) as {
  input: { question: string; answerExcerpt: string };
  body: Record<string, unknown>;
  timeoutMs: number;
};

const CONFIG = {
  model: 'claude-haiku-5-5',
  system: FOLLOW_UP_PROMPT,
  maxTokens: FOLLOW_UP_MAX_TOKENS,
  timeoutMs: FOLLOW_UP_TIMEOUT_MS,
  schema: FOLLOW_UP_SCHEMA,
};

const reply = (text: string, stopReason = 'end_turn') =>
  ({
    stop_reason: stopReason,
    content: [{ type: 'text', text }],
    usage: {
      input_tokens: 7,
      output_tokens: 9,
      cache_read_input_tokens: null,
      cache_creation_input_tokens: 0,
    },
  }) as unknown as Message;

const client = (outcome: Message | Error) => {
  const calls: unknown[] = [];
  const messages: CreateClient = {
    create: (body, options) => {
      calls.push([body, options]);
      return outcome instanceof Error ? Promise.reject(outcome) : Promise.resolve(outcome);
    },
  };
  return { messages, calls };
};

describe('anthropic-follow-up-suggester v1', () => {
  it('sends exactly the fixture body (thinking disabled, effort low, structured JSON output, max_tokens 400) with a 15 s timeout', async () => {
    const { model, max_tokens, thinking, output_config, messages } = FIXTURE.body;
    const expected = JSON.stringify({
      model,
      max_tokens,
      system: FOLLOW_UP_PROMPT,
      thinking,
      output_config,
      messages,
    });
    const { messages: api, calls } = client(reply('{"followUps":[]}'));

    await anthropicFollowUpSuggester(api, CONFIG, httpDeps().log).suggest(FIXTURE.input);

    expect(JSON.stringify(toFollowUpParams(CONFIG, FIXTURE.input))).toBe(expected);
    expect(calls).toEqual([
      [toFollowUpParams(CONFIG, FIXTURE.input), { timeout: FIXTURE.timeoutMs }],
    ]);
  });

  it('returns the trimmed questions and the call usage', async () => {
    const { messages } = client(reply('{"followUps":[" What is laches? ","Is it a defence?"]}'));

    expect(
      await anthropicFollowUpSuggester(messages, CONFIG, httpDeps().log).suggest(FIXTURE.input),
    ).toEqual({
      questions: ['What is laches?', 'Is it a defence?'],
      usage: { input: 7, output: 9, cacheRead: 0, cacheWrite: 0 },
    });
  });

  it.each([
    ['not JSON', 'nope'],
    ['six questions', JSON.stringify({ followUps: ['a', 'b', 'c', 'd', 'e', 'f'] })],
    ['a 121-char question', JSON.stringify({ followUps: ['x'.repeat(121)] })],
    ['an extra key', JSON.stringify({ followUps: [], why: 'x' })],
  ])('gives [] (keeping usage) and one warn for %s', async (_label, text) => {
    const deps = httpDeps();
    const { messages } = client(reply(text));

    const result = await anthropicFollowUpSuggester(messages, CONFIG, deps.log).suggest(
      FIXTURE.input,
    );

    expect(result).toEqual({
      questions: [],
      usage: { input: 7, output: 9, cacheRead: 0, cacheWrite: 0 },
    });
    expect(deps.lines).toEqual([
      {
        level: 'warn',
        event: 'followups.failed',
        operation: 'followups',
        outcome: 'invalid_output',
        model: 'claude-haiku-5-5',
      },
    ]);
  });

  it('gives [] with no usage and one warn when the call fails (e.g. the 15 s timeout)', async () => {
    const deps = httpDeps();
    const { messages } = client(
      Object.assign(new Error('t'), { name: 'APIConnectionTimeoutError' }),
    );

    expect(
      await anthropicFollowUpSuggester(messages, CONFIG, deps.log).suggest(FIXTURE.input),
    ).toEqual({ questions: [], usage: null });
    expect(deps.lines).toEqual([
      {
        level: 'warn',
        event: 'followups.failed',
        operation: 'followups',
        outcome: 'call_failed',
        'error.type': 'APIConnectionTimeoutError',
      },
    ]);
  });

  it.each(['max_tokens', 'refusal'])(
    'treats stop_reason %s as invalid output, even with parseable JSON',
    async (stop) => {
      const deps = httpDeps();
      const { messages } = client(reply('{"followUps":["What is laches?"]}', stop));

      const result = await anthropicFollowUpSuggester(messages, CONFIG, deps.log).suggest(
        FIXTURE.input,
      );

      expect(result.questions).toEqual([]);
      expect(deps.lines[0]).toMatchObject({ level: 'warn', outcome: 'invalid_output' });
    },
  );
});
