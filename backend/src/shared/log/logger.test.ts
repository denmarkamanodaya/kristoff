import { describe, expect, it } from 'vitest';
import { createLogger, LOG_FIELDS, redact } from './logger.js';

describe('logger', () => {
  it('pins the allowlist: no prompt, output, transcript, caption, query or key field', () => {
    expect([...LOG_FIELDS]).toMatchInlineSnapshot(`
      [
        "requestId",
        "operation",
        "outcome",
        "durationMs",
        "code",
        "principalHash",
        "stage",
        "model",
        "inputTokens",
        "outputTokens",
        "latencyMs",
        "count",
        "version",
        "http.route",
        "http.request.method",
        "http.response.status_code",
        "error.type",
        "trace_id",
        "step",
      ]
    `);
  });

  it('drops every field that is not allowlisted, and non-scalar values', () => {
    const out = redact({ prompt: 'secret', transcript: 'x', code: 'OK', count: 2, outcome: {} });

    expect(out).toEqual({ code: 'OK', count: 2 });
  });

  it('writes one JSON line per call at or above the level', () => {
    const lines: string[] = [];
    const log = createLogger({ level: 'info', write: (line) => lines.push(line) });

    log.debug('hidden');
    log.info('caption.done', { outcome: 'ok', model: 'claude-haiku-5-5' });

    expect(lines.map((l) => JSON.parse(l) as unknown)).toEqual([
      { level: 'info', event: 'caption.done', outcome: 'ok', model: 'claude-haiku-5-5' },
    ]);
  });
});
