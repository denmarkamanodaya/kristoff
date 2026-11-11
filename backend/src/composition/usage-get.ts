// Composition root of the `usage-get` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { systemClock } from '../adapters/system-clock/index.js';
import { getUsage } from '../usecases/get-usage/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the usage-get handler.
 * @returns Handler plumbing and the get-usage use case.
 * @throws Error when the env is invalid.
 */
export const usageGetDeps = async () => {
  const env = parseEnv('usage-get', process.env);
  return Promise.resolve({
    http: baseDeps('usage-get', env.STAGE).http,
    getUsage: getUsage({ meter: adapters.meter(env.TABLE_NAME), clock: systemClock }),
  });
};
