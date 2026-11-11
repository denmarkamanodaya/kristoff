import { PassThrough } from 'node:stream';
import { afterEach, describe, expect, it } from 'vitest';
import { sse, streamingEntry } from './lambda-stream.js';

const host = globalThis as unknown as { awslambda?: unknown };
const original = host.awslambda;

afterEach(() => {
  host.awslambda = original;
});

describe('lambda-stream', () => {
  it('formats one SSE event', () => {
    expect(sse('text', { text: 'a' })).toBe('event: text\ndata: {"text":"a"}\n\n');
  });

  it('is undefined off-Lambda, also when the SDK left an empty awslambda global', () => {
    host.awslambda = {};

    expect(streamingEntry(() => Promise.resolve())).toBeUndefined();
  });

  it('wraps with streamifyResponse and opens the response through HttpResponseStream.from', async () => {
    const metas: unknown[] = [];
    host.awslambda = {
      streamifyResponse: (h: unknown) => h,
      HttpResponseStream: { from: (s: PassThrough, meta: unknown) => (metas.push(meta), s) },
    };
    const entry = streamingEntry((_event, open) => {
      const w = open(200, { a: 'b' });
      w.write('x');
      w.end();
      return Promise.resolve();
    }) as unknown as (e: unknown, s: PassThrough) => Promise<void>;
    const stream = new PassThrough();
    const chunks: string[] = [];
    stream.on('data', (c: Buffer) => chunks.push(c.toString()));

    await entry({}, stream);

    expect([metas, chunks, stream.writableEnded]).toEqual([
      [{ statusCode: 200, headers: { a: 'b' } }],
      ['x'],
      true,
    ]);
  });

  it('ends the raw stream when the run never opened a response', async () => {
    host.awslambda = {
      streamifyResponse: (h: unknown) => h,
      HttpResponseStream: { from: () => undefined },
    };
    const entry = streamingEntry(() =>
      Promise.reject(new Error('cold start failed')),
    ) as unknown as (e: unknown, s: PassThrough) => Promise<void>;
    const stream = new PassThrough();

    await expect(entry({}, stream)).rejects.toThrow('cold start failed');
    expect(stream.writableEnded).toBe(true);
  });
});
