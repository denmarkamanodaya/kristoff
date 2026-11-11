import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { LAMBDA_ENV, parseEnv } from './env.js';

const SCOPE = JSON.parse(
  readFileSync(new URL('../../lambda-scope.json', import.meta.url), 'utf8'),
) as {
  functions: Record<string, { env: string[]; ssm: string[] }>;
};

const FULL = {
  STAGE: 'prod',
  OWNER_ID: 'kristoff',
  TABLE_NAME: 'kristoff-prod-core',
  DOCS_BUCKET: 'kristoff-prod-docs-123',
  MODEL_IDS: 'claude-sonnet-5-5, claude-opus-5-5,claude-haiku-5-5',
  MAX_TOKENS: '4096',
  MAX_ATTACHED_DOCS: '5',
  MAX_HISTORY_TURNS: '20',
  DAILY_QUESTION_CAP: '100',
  SESSION_TTL_SECONDS: '43200',
  ANTHROPIC_TIMEOUT_MS: '120000',
  SSM_ANTHROPIC_KEY: '/kristoff/prod/anthropic-api-key',
  SSM_TOTP_SECRET: '/kristoff/prod/totp-secret',
  SSM_SESSION_KEY: '/kristoff/prod/session-hmac-key',
  SSM_AI_ENABLED: '/kristoff/prod/ai-enabled',
};

describe('composition env', () => {
  it('declares exactly the Lambdas lambda-scope.json lists', () => {
    expect(Object.keys(LAMBDA_ENV).sort()).toEqual(Object.keys(SCOPE.functions).sort());
  });

  it.each(Object.entries(SCOPE.functions))(
    '%s parses exactly the env keys lambda-scope.json lists',
    (name, scope) => {
      const schema = LAMBDA_ENV[name as keyof typeof LAMBDA_ENV];

      expect(Object.keys(schema.shape).sort()).toEqual([...scope.env].sort());
      expect(scope.ssm.every((key) => scope.env.includes(key))).toBe(true);
    },
  );

  it('parses numbers', () => {
    const env = parseEnv('ask-stream', FULL);

    expect([env.MAX_TOKENS, env.ANTHROPIC_TIMEOUT_MS]).toEqual([4096, 120000]);
  });

  it('fails fast naming the bad keys, never their values', () => {
    const bad = {
      ...FULL,
      MAX_TOKENS: 'lots',
      ANTHROPIC_TIMEOUT_MS: '999999',
      SSM_AI_ENABLED: 'secret-value',
    };

    expect(() => parseEnv('ask-stream', bad)).toThrow(
      'invalid env for ask-stream: MAX_TOKENS, ANTHROPIC_TIMEOUT_MS, SSM_AI_ENABLED',
    );
    expect(() => parseEnv('ask-stream', bad)).not.toThrow(/secret-value/);
  });

  it('parses MODEL_IDS into a trimmed list, first = default', () => {
    expect(parseEnv('config-get', FULL).MODEL_IDS).toEqual([
      'claude-sonnet-5-5',
      'claude-opus-5-5',
      'claude-haiku-5-5',
    ]);
  });

  it.each([
    ['empty', ''],
    ['a duplicate', 'a-1,a-1'],
    ['six ids', 'a,b,c,d,e,f'],
    ['an upper-case id', 'Claude'],
  ])('refuses MODEL_IDS with %s', (_label, value) => {
    expect(() => parseEnv('config-get', { ...FULL, MODEL_IDS: value })).toThrow('MODEL_IDS');
  });

  it('refuses any stage but prod', () => {
    expect(() => parseEnv('docs-list', { ...FULL, STAGE: 'dev' })).toThrow('STAGE');
  });
});
