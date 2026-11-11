// Builders for HTTP API v2 events behind the Lambda authorizer, and handler deps that collect
// log lines.
import type { HttpEvent } from '../../src/shared/http/http.js';

/** An authorized request; pass `sub: undefined` to drop the authorizer context. */
export const httpEvent = (options: {
  routeKey: string;
  body?: unknown;
  rawBody?: string;
  sub?: string | undefined;
  query?: Record<string, string>;
  path?: Record<string, string>;
}): HttpEvent =>
  ({
    version: '2.0',
    routeKey: options.routeKey,
    rawPath: options.routeKey.split(' ')[1] ?? '/',
    rawQueryString: '',
    headers: {},
    ...(options.query ? { queryStringParameters: options.query } : {}),
    ...(options.path ? { pathParameters: options.path } : {}),
    isBase64Encoded: false,
    body:
      options.rawBody ?? (options.body === undefined ? undefined : JSON.stringify(options.body)),
    requestContext: {
      requestId: 'req-1',
      routeKey: options.routeKey,
      authorizer:
        'sub' in options && options.sub === undefined
          ? undefined
          : { lambda: { sub: options.sub ?? 'kristoff' } },
    },
  }) as unknown as HttpEvent;

/** Fixed time source and a logger that collects lines. */
export const httpDeps = () => {
  const lines: Record<string, unknown>[] = [];
  return {
    lines,
    log: {
      debug: () => undefined,
      info: (event: string, fields = {}) => lines.push({ level: 'info', event, ...fields }),
      warn: (event: string, fields = {}) => lines.push({ level: 'warn', event, ...fields }),
      error: (event: string, fields = {}) => lines.push({ level: 'error', event, ...fields }),
    },
    now: () => 1000,
  };
};

/** Parses a handler response body. */
export const bodyOf = (res: { body?: string | undefined }): unknown =>
  JSON.parse(res.body ?? 'null');
