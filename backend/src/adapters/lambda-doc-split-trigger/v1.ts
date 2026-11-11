// LambdaDocSplitTrigger: the DocSplitTrigger port as an asynchronous Lambda invoke of docs-split
// (docs/contract.md "v2 Large PDFs"). The payload carries ids only. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { InvokeCommand, type LambdaClient } from '@aws-sdk/client-lambda';
import type { DocSplitTrigger } from '../../ports/doc-split-trigger.js';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { upstreamError } from '../../shared/errors/vendor-error.js';

/** The client and the docs-split function name (env `DOCS_SPLIT_FUNCTION`). */
export interface LambdaTarget {
  readonly client: Pick<LambdaClient, 'send'>;
  readonly functionName: string;
}

/**
 * Builds the Lambda doc-split trigger.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Client and function name.
 * @returns The DocSplitTrigger; a rejected or non-202 invoke is UPSTREAM_UNAVAILABLE.
 */
export const lambdaDocSplitTrigger = ({ client, functionName }: LambdaTarget): DocSplitTrigger => ({
  start: async (uid, docId) => {
    try {
      const out = await client.send(
        new InvokeCommand({
          FunctionName: functionName,
          InvocationType: 'Event',
          Payload: Buffer.from(JSON.stringify({ uid, docId })),
        }),
      );
      return out.StatusCode === 202
        ? ok(undefined)
        : err(
            appError('UPSTREAM_UNAVAILABLE', 'docs-split was not accepted', {
              operation: 'lambda.Invoke',
            }),
          );
    } catch (error) {
      return err(upstreamError('lambda.Invoke', error, 'docs-split could not be started'));
    }
  },
});
