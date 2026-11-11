// The one env schema (core/dry.md "Env keys"; docs/contract.md "Config"). Each Lambda parses only
// the keys lambda-scope.json lists for it (env.test.ts pins that). Parsing fails fast at cold start
// with the bad key NAMES, never values. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';

const name = z.string().min(1).max(256);
const ssmName = z.string().regex(/^\/[A-Za-z0-9_./-]{1,255}$/);
const modelId = z.string().regex(/^[a-z0-9.-]{1,64}$/);
const int = (min: number, max: number) => z.coerce.number().int().min(min).max(max);

const fields = {
  STAGE: z.enum(['prod']),
  OWNER_ID: z.string().regex(/^[A-Za-z0-9_-]{1,64}$/),
  TABLE_NAME: name,
  DOCS_BUCKET: name,
  // Allowed model ids, first = default (docs/contract.md "Model choice").
  MODEL_IDS: z
    .string()
    .max(1024)
    .transform((list) => list.split(',').map((id) => id.trim()))
    .pipe(
      z
        .tuple([modelId], modelId)
        .refine(
          (ids) => ids.length <= 5 && new Set(ids).size === ids.length,
          'at most 5 unique ids',
        ),
    ),
  MAX_TOKENS: int(1, 64_000),
  MAX_ATTACHED_DOCS: int(1, 20),
  MAX_HISTORY_TURNS: int(0, 100),
  DAILY_QUESTION_CAP: int(1, 10_000),
  SESSION_TTL_SECONDS: int(300, 86_400),
  // Two attempts (MODEL_MAX_RETRIES = 1) must fit in the 300 s stream Lambda.
  ANTHROPIC_TIMEOUT_MS: int(1_000, 140_000),
  SSM_ANTHROPIC_KEY: ssmName,
  SSM_TOTP_SECRET: ssmName,
  SSM_SESSION_KEY: ssmName,
  SSM_AI_ENABLED: ssmName,
};

const pick = <K extends keyof typeof fields>(...keys: K[]) =>
  z.object(Object.fromEntries(keys.map((k) => [k, fields[k]])) as { [P in K]: (typeof fields)[P] });

/** Env schema per Lambda name (docs/contract.md "Build artefacts"). */
export const LAMBDA_ENV = {
  authorizer: pick('STAGE', 'OWNER_ID', 'SSM_SESSION_KEY'),
  'auth-verify': pick(
    'STAGE',
    'OWNER_ID',
    'TABLE_NAME',
    'SESSION_TTL_SECONDS',
    'SSM_TOTP_SECRET',
    'SSM_SESSION_KEY',
  ),
  'config-get': pick('STAGE', 'MODEL_IDS', 'MAX_ATTACHED_DOCS', 'DAILY_QUESTION_CAP'),
  'docs-list': pick('STAGE', 'TABLE_NAME'),
  'docs-create': pick('STAGE', 'TABLE_NAME', 'DOCS_BUCKET'),
  'docs-confirm': pick('STAGE', 'TABLE_NAME', 'DOCS_BUCKET'),
  'docs-rename': pick('STAGE', 'TABLE_NAME'),
  'docs-link': pick('STAGE', 'TABLE_NAME', 'DOCS_BUCKET'),
  'chats-list': pick('STAGE', 'TABLE_NAME'),
  'chats-create': pick('STAGE', 'TABLE_NAME', 'MAX_ATTACHED_DOCS'),
  'chats-get': pick('STAGE', 'TABLE_NAME'),
  'chats-update': pick('STAGE', 'TABLE_NAME', 'MAX_ATTACHED_DOCS'),
  'usage-get': pick('STAGE', 'TABLE_NAME'),
  'ask-stream': pick(
    'STAGE',
    'OWNER_ID',
    'TABLE_NAME',
    'DOCS_BUCKET',
    'MODEL_IDS',
    'MAX_TOKENS',
    'MAX_HISTORY_TURNS',
    'DAILY_QUESTION_CAP',
    'ANTHROPIC_TIMEOUT_MS',
    'SSM_ANTHROPIC_KEY',
    'SSM_SESSION_KEY',
    'SSM_AI_ENABLED',
  ),
} as const;

/** A Lambda name. */
export type LambdaName = keyof typeof LAMBDA_ENV;

/** The parsed env of one Lambda. */
export type EnvOf<N extends LambdaName> = z.infer<(typeof LAMBDA_ENV)[N]>;

/**
 * Parses one Lambda's env.
 * @param lambda - The Lambda name.
 * @param source - `process.env` (or a test object).
 * @returns The typed env.
 * @throws Error naming the invalid keys (never their values) when the env is invalid.
 */
export const parseEnv = <N extends LambdaName>(
  lambda: N,
  source: Readonly<Record<string, string | undefined>>,
): EnvOf<N> => {
  const parsed = LAMBDA_ENV[lambda].safeParse(source);
  if (parsed.success) return parsed.data as EnvOf<N>;
  const keys = [...new Set(parsed.error.issues.map((i) => String(i.path[0] ?? '?')))];
  throw new Error(`invalid env for ${lambda}: ${keys.join(', ')}`);
};
