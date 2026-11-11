// Composition root of the `authorizer` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { systemClock } from '../adapters/system-clock/index.js';
import { verifySession } from '../usecases/verify-session/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the authorizer.
 * @returns Logger and the verify-session use case.
 * @throws Error when the env is invalid.
 */
export const authorizerDeps = async () => {
  const env = parseEnv('authorizer', process.env);
  const { log } = baseDeps('authorizer', env.STAGE);
  const secrets = adapters.secrets({ 'session-key': env.SSM_SESSION_KEY });
  return Promise.resolve({
    log,
    verifySession: verifySession({ secrets, clock: systemClock, ownerId: env.OWNER_ID }),
  });
};
