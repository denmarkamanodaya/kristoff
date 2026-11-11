import { describe, expect, it } from 'vitest';
import { CitationWire, DoneEvent, TextEvent } from '../../contracts/shapes.js';
import { ErrorEnvelope } from '../../contracts/errors.js';
import type { AnswerSink } from '../../ports/answer-sink.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { aMessage } from '../../../test/support/builders.js';
import { httpDeps } from '../../../test/support/http-events.js';
import { createAskStreamHandler, type AskStreamDeps } from './v1.js';

const CITATION = {
  n: 1,
  docId: 'aaaaaaaaaaaaaaaa',
  docTitle: 'Cruz',
  page: 2,
  endPage: 2,
  quote: 'q',
};
const USAGE = { input: 1, output: 2, cacheRead: 3, cacheWrite: 4 };
const ORIGIN = 'https://kristoff.denmarkamanodaya.dev';

const urlEvent = (over: Record<string, unknown> = {}) => ({
  rawPath: '/v1/chats/cccccccccccccccc/ask',
  body: JSON.stringify({ text: 'What is estoppel?' }),
  isBase64Encoded: false,
  headers: { authorization: 'Bearer a.b', origin: ORIGIN },
  requestContext: { requestId: 'r-1', http: { method: 'POST' } },
  ...over,
});

const recorder = () => {
  const out: {
    status?: number;
    headers?: Record<string, string>;
    chunks: string[];
    ended: number;
  } = { chunks: [], ended: 0 };
  const open = (status: number, headers: Record<string, string>) => {
    out.status = status;
    out.headers = headers;
    return { write: (c: string) => void out.chunks.push(c), end: () => void (out.ended += 1) };
  };
  return { out, open };
};

const deps = (
  ask: AskStreamDeps['askQuestion'],
  verified = true,
): AskStreamDeps & { lines: unknown[] } => {
  const http = httpDeps();
  return {
    log: http.log,
    lines: http.lines,
    now: http.now,
    verifySession: () =>
      Promise.resolve(verified ? ok({ uid: 'kristoff' }) : err(appError('UNAUTHENTICATED', 'no'))),
    askQuestion: ask,
  };
};

const parseSse = (chunks: string[]) =>
  chunks.map((c) => {
    const [, event, data] = /^event: (\w+)\ndata: (.*)\n\n$/s.exec(c) ?? [];
    return { event, data: JSON.parse(data ?? 'null') as unknown };
  });

describe('ask-stream handler v1', () => {
  it('streams text, citation and done as SSE, with no CORS headers (Terraform owns them)', async () => {
    const seen: unknown[] = [];
    const message = aMessage({
      role: 'assistant',
      text: 'Bars [1]',
      citations: [CITATION],
      usage: USAGE,
    });
    const handler = createAskStreamHandler(
      deps((p, chatId, text, sink: AnswerSink) => {
        seen.push([p, chatId, text]);
        sink.open();
        sink.text('Bars');
        sink.citation(CITATION);
        return Promise.resolve(ok({ message, usage: USAGE }));
      }),
    );
    const { out, open } = recorder();

    await handler(urlEvent(), open);

    expect(seen).toEqual([[{ uid: 'kristoff' }, 'cccccccccccccccc', 'What is estoppel?']]);
    expect([out.status, out.headers]).toEqual([
      200,
      {
        'x-request-id': 'r-1',
        'content-type': 'text/event-stream',
        'cache-control': 'no-cache',
      },
    ]);
    const events = parseSse(out.chunks);
    expect(events.map((e) => e.event)).toEqual(['text', 'citation', 'done']);
    expect([
      TextEvent.parse(events[0]?.data),
      CitationWire.parse(events[1]?.data),
      DoneEvent.parse(events[2]?.data),
    ]).toEqual([{ text: 'Bars' }, CITATION, { message, usage: USAGE }]);
    expect(out.ended).toBe(1);
  });

  it('answers a pre-stream failure as the JSON envelope with its status and Retry-After', async () => {
    const handler = createAskStreamHandler(
      deps(() =>
        Promise.resolve(err(appError('DAILY_CAP_REACHED', 'cap', { retryAfterSeconds: 7200 }))),
      ),
    );
    const { out, open } = recorder();

    await handler(urlEvent(), open);

    expect([out.status, out.headers?.['retry-after'], out.headers?.['content-type']]).toEqual([
      429,
      '7200',
      'application/json',
    ]);
    expect(ErrorEnvelope.parse(JSON.parse(out.chunks.join('')))).toEqual({
      error: { code: 'DAILY_CAP_REACHED', message: 'cap', requestId: 'r-1' },
    });
  });

  it('answers 401 without calling ask-question when the session does not verify', async () => {
    const handler = createAskStreamHandler(deps(() => Promise.reject(new Error('called')), false));
    const { out, open } = recorder();

    await handler(urlEvent(), open);

    expect(out.status).toBe(401);
  });

  it.each([
    ['a GET', { requestContext: { requestId: 'r-1', http: { method: 'GET' } } }],
    ['another path', { rawPath: '/v1/chats/xyz/ask' }],
    ['an empty question', { body: JSON.stringify({ text: '' }) }],
    ['an 8001-char question', { body: JSON.stringify({ text: 'x'.repeat(8001) }) }],
  ])('answers 400 VALIDATION_FAILED for %s', async (_label, over) => {
    const handler = createAskStreamHandler(deps(() => Promise.reject(new Error('called'))));
    const { out, open } = recorder();

    await handler(urlEvent(over), open);

    expect(out.status).toBe(400);
  });

  it('sends an error event after the stream opened', async () => {
    const handler = createAskStreamHandler(
      deps((_p, _c, _t, sink) => {
        sink.open();
        sink.text('partial');
        return Promise.resolve(err(appError('UPSTREAM_UNAVAILABLE', 'model down')));
      }),
    );
    const { out, open } = recorder();

    await handler(
      urlEvent({ headers: { authorization: 'Bearer a.b', origin: 'https://evil.example' } }),
      open,
    );
    expect(parseSse(out.chunks).at(-1)).toEqual({
      event: 'error',
      data: { error: { code: 'UPSTREAM_UNAVAILABLE', message: 'model down', requestId: 'r-1' } },
    });
  });

  it('logs one outcome line and tolerates a non-object event', async () => {
    const d = deps(() => Promise.reject(new Error('called')));
    const { open } = recorder();

    await createAskStreamHandler(d)('garbage', open);

    expect(d.lines).toEqual([
      {
        level: 'info',
        event: 'stream.request',
        operation: 'ask-stream',
        requestId: 'unknown',
        outcome: 'VALIDATION_FAILED',
        durationMs: 0,
      },
    ]);
  });
});
