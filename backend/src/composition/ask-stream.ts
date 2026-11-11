// Composition root of the `ask-stream` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import Anthropic from '@anthropic-ai/sdk';
import { anthropicAnswerModel } from '../adapters/anthropic-answer-model/index.js';
import { anthropicFollowUpSuggester } from '../adapters/anthropic-follow-up-suggester/index.js';
import { cryptoIdGenerator } from '../adapters/crypto-id-generator/index.js';
import { ssmFlagReader } from '../adapters/ssm/ssm-flag-reader/index.js';
import { systemClock } from '../adapters/system-clock/index.js';
import {
  FOLLOW_UP_MAX_TOKENS,
  FOLLOW_UP_PROMPT,
  FOLLOW_UP_SCHEMA,
  FOLLOW_UP_TIMEOUT_MS,
  MODEL_MAX_RETRIES,
  SYSTEM_PROMPT,
} from '../contracts/model-config.js';
import { askQuestion } from '../usecases/ask-question/index.js';
import { verifySession } from '../usecases/verify-session/index.js';
import type { Logger } from '../shared/log/logger.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/** The follow-up suggester on the same client, with the contract's prompt and limits. */
const followUpsFor = (client: Anthropic, model: string, log: Logger) =>
  anthropicFollowUpSuggester(
    client.messages,
    {
      model,
      system: FOLLOW_UP_PROMPT,
      maxTokens: FOLLOW_UP_MAX_TOKENS,
      timeoutMs: FOLLOW_UP_TIMEOUT_MS,
      schema: FOLLOW_UP_SCHEMA,
    },
    log,
  );

/**
 * Wires the ask-stream handler. The Anthropic key is read once per container, at cold start.
 * @returns Logger, verify-session and ask-question.
 * @throws Error when the env is invalid or the API key cannot be read.
 */
export const askStreamDeps = async () => {
  const env = parseEnv('ask-stream', process.env);
  const { log, http } = baseDeps('ask-stream', env.STAGE);
  const secrets = adapters.secrets({
    'anthropic-key': env.SSM_ANTHROPIC_KEY,
    'session-key': env.SSM_SESSION_KEY,
  });
  const apiKey = await secrets.get('anthropic-key');
  if (!apiKey.ok) throw apiKey.error;
  const client = new Anthropic({
    apiKey: apiKey.value,
    timeout: env.ANTHROPIC_TIMEOUT_MS,
    maxRetries: MODEL_MAX_RETRIES,
  });
  return {
    log,
    now: http.now,
    verifySession: verifySession({ secrets, clock: systemClock, ownerId: env.OWNER_ID }),
    askQuestion: askQuestion({
      chats: adapters.chats(env.TABLE_NAME),
      docs: adapters.docs(env.TABLE_NAME),
      files: adapters.files(env.DOCS_BUCKET),
      model: anthropicAnswerModel(client.messages, SYSTEM_PROMPT),
      followUps: followUpsFor(client, env.FOLLOWUP_MODEL_ID, log),
      meter: adapters.meter(env.TABLE_NAME),
      flags: ssmFlagReader(adapters.ssm(), env.SSM_AI_ENABLED),
      ids: cryptoIdGenerator(() => systemClock.now()),
      clock: systemClock,
      config: {
        modelIds: env.MODEL_IDS,
        maxTokens: env.MAX_TOKENS,
        maxHistoryTurns: env.MAX_HISTORY_TURNS,
        dailyQuestionCap: env.DAILY_QUESTION_CAP,
      },
    }),
  };
};
