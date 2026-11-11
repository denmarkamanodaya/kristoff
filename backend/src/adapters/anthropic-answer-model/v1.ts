// AnthropicAnswerModel: the AnswerModel port on the Anthropic Messages streaming API
// (docs/contract.md "Model call"; stack/ai-features.md). Docs go as `url` document sources with
// citations on and the cache breakpoint on the last doc; SDK events and errors are translated
// here and never cross the port. Prompts and outputs are never logged.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type {
  ContentBlockParam,
  Message,
  MessageParam,
  MessageStreamEvent,
  MessageStreamParams,
} from '@anthropic-ai/sdk/resources/messages/messages';
import type {
  AnswerEvent,
  CitationSource,
  ModelRequest,
  ModelTurn,
  TurnBlock,
} from '../../domain/chats/model-request.js';
import type { AnswerModel } from '../../ports/answer-model.js';
import { upstreamError } from '../../shared/errors/vendor-error.js';
import { err, ok } from '../../shared/result/result.js';

/** A started stream: its events, then the final message. */
interface StreamLike extends AsyncIterable<MessageStreamEvent> {
  finalMessage(): Promise<Message>;
}

/** The slice of `client.messages` this adapter calls (a stub in tests). */
export interface MessagesClient {
  stream(body: MessageStreamParams): StreamLike;
}

const toBlock = (block: TurnBlock): ContentBlockParam =>
  block.kind === 'text'
    ? { type: 'text', text: block.text }
    : {
        type: 'document',
        source: { type: 'url', url: block.url },
        title: block.title,
        citations: { enabled: true },
        ...(block.isCacheBreakpoint ? { cache_control: { type: 'ephemeral' } } : {}),
      };

const toMessage = (turn: ModelTurn): MessageParam => {
  const [only] = turn.blocks;
  const isPlainText = turn.blocks.length === 1 && only?.kind === 'text';
  return { role: turn.role, content: isPlainText ? only.text : turn.blocks.map(toBlock) };
};

/**
 * The exact request body sent to the API (exported for the wire test).
 * @param request - Domain request.
 * @param system - The system prompt.
 * @returns The Messages API params.
 */
export const toParams = (request: ModelRequest, system: string): MessageStreamParams => ({
  model: request.model,
  max_tokens: request.maxTokens,
  system,
  messages: request.turns.map(toMessage),
});

type Delta = Extract<MessageStreamEvent, { type: 'content_block_delta' }>['delta'];

const toCitation = (delta: Extract<Delta, { type: 'citations_delta' }>): CitationSource => {
  const c = delta.citation;
  const isPage = c.type === 'page_location';
  const isDoc = 'document_index' in c;
  return {
    documentIndex: isDoc ? c.document_index : -1,
    documentTitle: isDoc ? c.document_title : null,
    startPage: isPage ? c.start_page_number : null,
    endPageExclusive: isPage ? c.end_page_number : null,
    citedText: c.cited_text,
  };
};

const toEvent = (event: MessageStreamEvent): AnswerEvent | undefined => {
  if (event.type !== 'content_block_delta') return undefined;
  if (event.delta.type === 'text_delta') return { kind: 'text', text: event.delta.text };
  if (event.delta.type === 'citations_delta')
    return { kind: 'citation', source: toCitation(event.delta) };
  return undefined;
};

/**
 * Builds the Anthropic answer model.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param messages - `client.messages` of a client built with an explicit timeout and retries.
 * @param system - The system prompt (src/contracts/model-config.ts).
 * @returns The AnswerModel; any SDK failure is `UPSTREAM_UNAVAILABLE` with the error type.
 */
export const anthropicAnswerModel = (messages: MessagesClient, system: string): AnswerModel => ({
  stream: async (request, onEvent) => {
    try {
      const stream = messages.stream(toParams(request, system));
      for await (const event of stream) {
        const mapped = toEvent(event);
        if (mapped) onEvent(mapped);
      }
      const final = await stream.finalMessage();
      const u = final.usage;
      return ok({
        usage: {
          input: u.input_tokens,
          output: u.output_tokens,
          cacheRead: u.cache_read_input_tokens ?? 0,
          cacheWrite: u.cache_creation_input_tokens ?? 0,
        },
        stopReason: final.stop_reason,
      });
    } catch (error) {
      return err(upstreamError('anthropic.messages.stream', error, 'the model call failed'));
    }
  },
});
