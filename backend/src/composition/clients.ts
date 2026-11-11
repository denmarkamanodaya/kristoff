// Shared wiring for every Lambda's composition root: SDK clients built once per container with
// explicit timeouts (stack/errors-and-resilience.md "Timeouts"), the logger, and adapter builders.
// Only composition modules import this file. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { DynamoDBClient } from '@aws-sdk/client-dynamodb';
import { S3Client } from '@aws-sdk/client-s3';
import { LambdaClient } from '@aws-sdk/client-lambda';
import { SSMClient } from '@aws-sdk/client-ssm';
import { DynamoDBDocumentClient } from '@aws-sdk/lib-dynamodb';
import { dynamoAuthStateStore } from '../adapters/dynamo/dynamo-auth-state-store/index.js';
import { dynamoChatStore } from '../adapters/dynamo/dynamo-chat-store/index.js';
import { dynamoDocStore } from '../adapters/dynamo/dynamo-doc-store/index.js';
import { dynamoProjectStore } from '../adapters/dynamo/dynamo-project-store/index.js';
import { dynamoUsageMeter } from '../adapters/dynamo/dynamo-usage-meter/index.js';
import { lambdaDocSplitTrigger } from '../adapters/lambda-doc-split-trigger/index.js';
import { s3FileStore } from '../adapters/s3-file-store/index.js';
import { ssmSecretSource } from '../adapters/ssm/ssm-secret-source/index.js';
import { systemClock } from '../adapters/system-clock/index.js';
import type { SecretName } from '../ports/secret-source.js';
import type { HttpDeps } from '../shared/http/http.js';
import { createLogger, type Logger } from '../shared/log/logger.js';

/** Per-attempt AWS timeouts in milliseconds; 3 attempts stay inside the 10 s Lambda timeout. */
const AWS_HTTP = { connectionTimeout: 1_000, requestTimeout: 2_500 } as const;
const AWS_MAX_ATTEMPTS = 3;

let doc: DynamoDBDocumentClient | undefined;
let s3: S3Client | undefined;
let ssm: SSMClient | undefined;
let s3Bulk: S3Client | undefined;
let lambda: LambdaClient | undefined;

/** docs-split moves up to 200 MiB per object: a longer per-attempt timeout, same retries. */
const AWS_BULK_HTTP = { connectionTimeout: 1_000, requestTimeout: 60_000 } as const;

const docClient = (): DynamoDBDocumentClient =>
  (doc ??= DynamoDBDocumentClient.from(
    new DynamoDBClient({ requestHandler: AWS_HTTP, maxAttempts: AWS_MAX_ATTEMPTS }),
    { marshallOptions: { removeUndefinedValues: true } },
  ));

const s3Client = (): S3Client =>
  // Checksums only when required: a default CRC32 header would break the browser's presigned PUT.
  (s3 ??= new S3Client({
    requestHandler: AWS_HTTP,
    maxAttempts: AWS_MAX_ATTEMPTS,
    requestChecksumCalculation: 'WHEN_REQUIRED',
    responseChecksumValidation: 'WHEN_REQUIRED',
  }));

const s3BulkClient = (): S3Client =>
  (s3Bulk ??= new S3Client({
    requestHandler: AWS_BULK_HTTP,
    maxAttempts: AWS_MAX_ATTEMPTS,
    requestChecksumCalculation: 'WHEN_REQUIRED',
    responseChecksumValidation: 'WHEN_REQUIRED',
  }));

const lambdaClient = (): LambdaClient =>
  (lambda ??= new LambdaClient({ requestHandler: AWS_HTTP, maxAttempts: AWS_MAX_ATTEMPTS }));

const ssmClient = (): SSMClient =>
  (ssm ??= new SSMClient({ requestHandler: AWS_HTTP, maxAttempts: AWS_MAX_ATTEMPTS }));

/**
 * The logger and the handler plumbing deps, logging the cold start once.
 * @param operation - The Lambda name.
 * @param stage - The stage from env.
 * @returns Logger plus `{log, now}` for the HTTP plumbing.
 */
export const baseDeps = (operation: string, stage: string): { log: Logger; http: HttpDeps } => {
  const log = createLogger({ level: 'info' });
  log.info('cold.start', { operation, stage, version: 1 });
  return { log, http: { log, now: () => systemClock.now() } };
};

/** Adapter builders bound to the shared clients. */
export const adapters = {
  docs: (table: string) => dynamoDocStore({ client: docClient(), table }),
  chats: (table: string) => dynamoChatStore({ client: docClient(), table }),
  projects: (table: string) => dynamoProjectStore({ client: docClient(), table }),
  authState: (table: string) => dynamoAuthStateStore({ client: docClient(), table }),
  meter: (table: string) => dynamoUsageMeter({ client: docClient(), table }),
  files: (bucket: string) =>
    s3FileStore({ client: s3Client(), bucket, now: () => systemClock.now() }),
  bulkFiles: (bucket: string) =>
    s3FileStore({ client: s3BulkClient(), bucket, now: () => systemClock.now() }),
  splitTrigger: (functionName: string) =>
    lambdaDocSplitTrigger({ client: lambdaClient(), functionName }),
  secrets: (names: Readonly<Partial<Record<SecretName, string>>>) =>
    ssmSecretSource({ client: ssmClient(), now: () => systemClock.now() }, names),
  ssm: () => ({ client: ssmClient(), now: () => systemClock.now() }),
};
