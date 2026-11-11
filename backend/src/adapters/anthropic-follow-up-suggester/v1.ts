// AnthropicFollowUpSuggester: the FollowUpSuggester port on one non-streaming Messages call with a
// structured JSON output (docs/contract.md "v2 Answer extras"; stack/ai-features.md: model output is
// untrusted, parsed with a bounded Zod schema). Any failure (call, timeout, refusal, invalid JSON)
// gives no questions and one `warn` line; it never becomes an error. Prompts and outputs are never
// logged. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type {
  Message,
  MessageCreateParamsNonStreaming,
} from '@anthropic-ai/sdk/resources/messages/messages';
import { z } from 'zod';
import { MAX_FOLLOW_UP, MAX_FOLLOW_UPS } from '../../domain/limits.js';
import type { FollowUps, Usage } from '../../domain/model.js';
import type { FollowUpSuggester } from '../../ports/follow-up-suggester.js';
import type { Logger } from '../../shared/log/logger.js';
import { errorTypeOf } from '../../shared/errors/vendor-error.js';

/** The slice of `client.messages` this adapter calls (a stub in tests). */
export interface CreateClient {
  create(body: MessageCreateParamsNonStreaming, options: { timeout: number }): Promise<Message>;
}

/** What the call is made with (all from src/contracts/model-config.ts and env). */
export interface FollowUpCallConfig {
  readonly model: string;
  readonly system: string;
  readonly maxTokens: number;
  readonly timeoutMs: number;
  readonly schema: Readonly<Record<string, unknown>>;
}

const Output = z
  .object({ followUps: z.array(z.string().trim().min(1).max(MAX_FOLLOW_UP)).max(MAX_FOLLOW_UPS) })
  .strict();

/**
 * The exact request body (exported for the wire test).
 * @param config - Model, prompt and limits.
 * @param input - The question and the answer excerpt (user-turn data, delimited).
 * @returns The Messages API params.
 */
export const toFollowUpParams = (
  config: FollowUpCallConfig,
  input: { readonly question: string; readonly answerExcerpt: string },
): MessageCreateParamsNonStreaming => ({
  model: config.model,
  max_tokens: config.maxTokens,
  system: config.system,
  // Haiku 5.5 thinks by default; with a 400-token cap that can starve the JSON, so thinking is off.
  thinking: { type: 'disabled' },
  output_config: {
    effort: 'low',
    format: { type: 'json_schema', schema: { ...config.schema } },
  },
  messages: [
    {
      role: 'user',
      content: `<question>\n${input.question}\n</question>\n<answer_start>\n${input.answerExcerpt}\n</answer_start>`,
    },
  ],
});

const toUsage = (message: Message): Usage => ({
  input: message.usage.input_tokens,
  output: message.usage.output_tokens,
  cacheRead: message.usage.cache_read_input_tokens ?? 0,
  cacheWrite: message.usage.cache_creation_input_tokens ?? 0,
});

/** Parses the first text block as the schema's JSON; undefined when it is missing or invalid. */
const questionsOf = (message: Message): string[] | undefined => {
  // A cut-off or refused reply is not trusted, even when its text happens to parse.
  if (message.stop_reason !== 'end_turn') return undefined;
  const block = message.content.find((b) => b.type === 'text');
  if (block?.type !== 'text') return undefined;
  try {
    const parsed = Output.safeParse(JSON.parse(block.text));
    return parsed.success ? parsed.data.followUps : undefined;
  } catch {
    return undefined;
  }
};

/**
 * Builds the Anthropic follow-up suggester.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param messages - `client.messages` of the Anthropic client.
 * @param config - Model (env `FOLLOWUP_MODEL_ID`), prompt, max tokens, timeout and schema.
 * @param log - Where the one `warn` per failure goes.
 * @returns The FollowUpSuggester.
 */
export const anthropicFollowUpSuggester = (
  messages: CreateClient,
  config: FollowUpCallConfig,
  log: Logger,
): FollowUpSuggester => ({
  suggest: async (input): Promise<FollowUps> => {
    let message: Message;
    try {
      message = await messages.create(toFollowUpParams(config, input), {
        timeout: config.timeoutMs,
      });
    } catch (error) {
      log.warn('followups.failed', {
        operation: 'followups',
        outcome: 'call_failed',
        'error.type': errorTypeOf(error),
      });
      return { questions: [], usage: null };
    }
    const questions = questionsOf(message);
    if (questions === undefined) {
      log.warn('followups.failed', {
        operation: 'followups',
        outcome: 'invalid_output',
        model: config.model,
      });
      return { questions: [], usage: toUsage(message) };
    }
    return { questions, usage: toUsage(message) };
  },
});
