// Response-streaming plumbing for the function-URL Lambda (docs/contract.md "Stream"): wraps the
// runtime's `awslambda.streamifyResponse` / `HttpResponseStream.from` (typed by @types/aws-lambda's
// global declaration) behind a small `open(status, headers)` writer, so the handler is testable
// without the runtime. Handlers only.
import type { StreamifyHandler } from 'aws-lambda';

/** A started response: write body chunks, then end once. */
export interface ResponseWriter {
  write(chunk: string): void;
  end(): void;
}

/** Starts the response with its status and headers (called at most once per invocation). */
export type OpenResponse = (status: number, headers: Record<string, string>) => ResponseWriter;

/**
 * Wraps `run` with `awslambda.streamifyResponse`. Outside the Lambda runtime (tests, the build's
 * import) the runtime members are absent and the entry is `undefined`.
 * @param run - Handles one event, starting the response through `open`.
 * @returns The streaming Lambda entry, or `undefined` off-Lambda.
 */
export const streamingEntry = (
  run: (event: unknown, open: OpenResponse) => Promise<void>,
): StreamifyHandler<unknown> | undefined => {
  // The AWS SDK sets `globalThis.awslambda = {}` off-Lambda, so check the members, not the global.
  const host = globalThis as unknown as { awslambda?: Partial<typeof awslambda> };
  const found = host.awslambda;
  if (found?.streamifyResponse === undefined || found.HttpResponseStream === undefined)
    return undefined;
  // Both members exist (checked above); call them on the runtime's own object, as AWS documents.
  const runtime = found as typeof awslambda;
  return runtime.streamifyResponse<unknown>(async (event, responseStream) => {
    const state = { isOpened: false };
    try {
      await run(event, (statusCode, headers) => {
        const stream = runtime.HttpResponseStream.from(responseStream, { statusCode, headers });
        state.isOpened = true;
        return { write: (chunk) => stream.write(chunk), end: () => stream.end() };
      });
    } finally {
      if (!state.isOpened) responseStream.end();
    }
  });
};

/**
 * Formats one server-sent event.
 * @param event - Event name (`text`, `citation`, `done`, `error`).
 * @param data - JSON-serialisable payload.
 * @returns `event: <name>\ndata: <json>\n\n`.
 */
export const sse = (event: string, data: unknown): string =>
  `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
