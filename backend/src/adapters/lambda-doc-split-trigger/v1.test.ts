import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../test/support/sdk-stub.js';
import { lambdaDocSplitTrigger } from './v1.js';

const trigger = (...replies: unknown[]) => {
  const { client, calls } = stubClient(...replies);
  const port = lambdaDocSplitTrigger({
    client,
    functionName: 'kristoff-prod-docs-split:live',
  });
  return { start: (uid: string, docId: string) => port.start(uid, docId), calls };
};

describe('lambda-doc-split-trigger v1', () => {
  it('invokes docs-split asynchronously (Event) with the ids only', async () => {
    const { start, calls } = trigger({ StatusCode: 202 });

    expect(await start('kristoff', 'aaaaaaaaaaaaaaaa')).toEqual({ ok: true, value: undefined });
    const input = calls[0]?.input as {
      FunctionName: string;
      InvocationType: string;
      Payload: Uint8Array;
    };
    expect([calls[0]?.command, input.FunctionName, input.InvocationType]).toEqual([
      'InvokeCommand',
      'kristoff-prod-docs-split:live',
      'Event',
    ]);
    expect(JSON.parse(Buffer.from(input.Payload).toString('utf8'))).toEqual({
      uid: 'kristoff',
      docId: 'aaaaaaaaaaaaaaaa',
    });
  });

  it('answers UPSTREAM_UNAVAILABLE for a non-202 status and a rejected invoke', async () => {
    const { start } = trigger({ StatusCode: 200 }, sdkError('TooManyRequestsException'));

    expect(await start('k', 'a')).toMatchObject({ error: { code: 'UPSTREAM_UNAVAILABLE' } });
    expect(await start('k', 'a')).toMatchObject({
      error: {
        code: 'UPSTREAM_UNAVAILABLE',
        details: { 'error.type': 'TooManyRequestsException' },
      },
    });
  });
});
