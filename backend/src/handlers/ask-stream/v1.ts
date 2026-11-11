// POST {STREAM_URL}/v1/chats/{id}/ask handler (docs/contract.md "Stream"): verifies the session
// itself (no authorizer on a function URL; CORS is the function URL's Terraform config), parses the body, runs ask-question, and writes SSE
// events. Before the stream opens, failures are the JSON envelope with a status; after it, an
// `error` event. Thin driving adapter. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { AskOutcome, Question } from '../../usecases/ask-question/index.js';
import type { Principal } from '../../domain/model.js';
import type { AnswerSink } from '../../ports/answer-sink.js';
import { AskRequest } from '../../contracts/requests.js';
import { askStreamDeps } from '../../composition/ask-stream.js';
import { appError, type AppError } from '../../shared/errors/app-error.js';
import { errorReply, parseJson, requestIdOf } from '../../shared/http/http.js';
import { lazyHandler } from '../../shared/lazy/lazy-handler.js';
import type { Logger } from '../../shared/log/logger.js';
import { err, ok, type Result } from '../../shared/result/result.js';
import {
  sse,
  streamingEntry,
  type OpenResponse,
  type ResponseWriter,
} from '../_shared/lambda-stream.js';

/** What the ask-stream handler needs. */
export interface AskStreamDeps {
  readonly log: Logger;
  readonly now: () => number;
  readonly verifySession: (
    authorization: string | undefined,
  ) => Promise<Result<Principal, AppError>>;
  readonly askQuestion: (
    p: Principal,
    chatId: string,
    question: Question,
    sink: AnswerSink,
  ) => Promise<Result<AskOutcome, AppError>>;
}

/** The function-URL event fields this handler reads (untrusted, so all optional). */
interface UrlEvent {
  readonly rawPath?: unknown;
  readonly body?: string;
  readonly isBase64Encoded?: boolean;
  readonly headers?: Record<string, string | undefined>;
  readonly requestContext?: { readonly http?: { readonly method?: unknown } };
}

const ASK_PATH = /^\/v1\/chats\/([a-f0-9]{16})\/ask$/;

/** Parses method, path and body; the chat id and question, or `VALIDATION_FAILED`. */
const parseRequest = (
  event: UrlEvent,
): Result<{ chatId: string; question: Question }, AppError> => {
  const chatId = typeof event.rawPath === 'string' ? ASK_PATH.exec(event.rawPath)?.[1] : undefined;
  if (event.requestContext?.http?.method !== 'POST' || chatId === undefined)
    return err(appError('VALIDATION_FAILED', 'use POST /v1/chats/{id}/ask'));
  const body = parseJson(event.body, event.isBase64Encoded === true, AskRequest);
  return body.ok
    ? ok({ chatId, question: { text: body.value.text, model: body.value.model } })
    : body;
};

/** A sink that opens the SSE response on first use and remembers the writer. */
const sseSink = (open: () => ResponseWriter) => {
  let writer: ResponseWriter | undefined;
  const sink: AnswerSink = {
    open: () => {
      writer ??= open();
    },
    text: (text) => writer?.write(sse('text', { text })),
    citation: (citation) => writer?.write(sse('citation', citation)),
  };
  return { sink, writer: () => writer };
};

/** Runs the use case when the caller and the request are valid; else the first failure. */
const answer = async (
  deps: AskStreamDeps,
  event: UrlEvent,
  sink: AnswerSink,
): Promise<Result<AskOutcome, AppError>> => {
  const principal = await deps.verifySession(event.headers?.authorization);
  if (!principal.ok) return principal;
  const request = parseRequest(event);
  if (!request.ok) return request;
  return deps.askQuestion(principal.value, request.value.chatId, request.value.question, sink);
};

/** Ends an opened stream with `done` or `error`. */
const finishStream = (
  writer: ResponseWriter,
  outcome: Result<AskOutcome, AppError>,
  requestId: string,
): void => {
  const last = outcome.ok
    ? sse('done', outcome.value)
    : sse('error', errorReply(outcome.error, requestId).envelope);
  writer.write(last);
  writer.end();
};

/** Answers a failure before the stream opened as the JSON envelope with its status. */
const replyJson = (
  open: OpenResponse,
  headers: Record<string, string>,
  error: AppError,
  requestId: string,
): void => {
  const reply = errorReply(error, requestId);
  const json = open(reply.status, {
    ...headers,
    ...reply.headers,
    'content-type': 'application/json',
  });
  json.write(JSON.stringify(reply.envelope));
  json.end();
};

const run = async (deps: AskStreamDeps, event: UrlEvent, open: OpenResponse): Promise<string> => {
  const requestId = requestIdOf(event);
  const base = { 'x-request-id': requestId };
  const sseHeaders = { ...base, 'content-type': 'text/event-stream', 'cache-control': 'no-cache' };
  const stream = sseSink(() => open(200, sseHeaders));
  const outcome = await answer(deps, event, stream.sink);
  const writer = stream.writer();
  if (writer !== undefined) finishStream(writer, outcome, requestId);
  else
    replyJson(
      open,
      base,
      outcome.ok ? appError('INTERNAL', 'stream never opened') : outcome.error,
      requestId,
    );
  return outcome.ok ? 'ok' : outcome.error.code;
};

/**
 * Builds the ask-stream handler.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param deps - Logger, clock, verify-session and ask-question.
 * @returns The handler: writes the response through `open` and logs one outcome line.
 */
export const createAskStreamHandler =
  (deps: AskStreamDeps) =>
  async (event: unknown, open: OpenResponse): Promise<void> => {
    const started = deps.now();
    const urlEvent: UrlEvent = typeof event === 'object' && event !== null ? event : {};
    const outcome = await run(deps, urlEvent, open);
    deps.log.info('stream.request', {
      operation: 'ask-stream',
      requestId: requestIdOf(event),
      outcome,
      durationMs: deps.now() - started,
    });
  };

const wired = lazyHandler(async () => {
  const handle = createAskStreamHandler(await askStreamDeps());
  return (input: { event: unknown; open: OpenResponse }) => handle(input.event, input.open);
});

/** Lambda entry (`index.handler`): the streaming wrapper; wires once per container. */
export const handler = streamingEntry((event, open) => wired({ event, open }));
