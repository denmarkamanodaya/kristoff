// Composition root of the `config-get` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { MODEL_LABELS } from '../contracts/model-config.js';
import { getConfig } from '../usecases/get-config/index.js';
import { baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the config-get handler.
 * @returns Handler plumbing and the get-config use case.
 * @throws Error when the env is invalid.
 */
export const configGetDeps = async () => {
  const env = parseEnv('config-get', process.env);
  return Promise.resolve({
    http: baseDeps('config-get', env.STAGE).http,
    getConfig: getConfig({
      modelIds: env.MODEL_IDS,
      labels: MODEL_LABELS,
      maxAttachedDocs: env.MAX_ATTACHED_DOCS,
      dailyQuestionCap: env.DAILY_QUESTION_CAP,
    }),
  });
};
