// Composition root of the `auth-verify` Lambda (docs/contract.md "Build artefacts"): reads env once, wires
// adapters to the use case. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { systemClock } from '../adapters/system-clock/index.js';
import { verifyOtp } from '../usecases/verify-otp/index.js';
import { adapters, baseDeps } from './clients.js';
import { parseEnv } from './env.js';

/**
 * Wires the auth-verify handler.
 * @returns Handler plumbing and the verify-otp use case.
 * @throws Error when the env is invalid.
 */
export const authVerifyDeps = async () => {
  const env = parseEnv('auth-verify', process.env);
  const secrets = adapters.secrets({
    'totp-secret': env.SSM_TOTP_SECRET,
    'session-key': env.SSM_SESSION_KEY,
  });
  return Promise.resolve({
    http: baseDeps('auth-verify', env.STAGE).http,
    verifyOtp: verifyOtp({
      authState: adapters.authState(env.TABLE_NAME),
      secrets,
      clock: systemClock,
      ownerId: env.OWNER_ID,
      sessionTtlSeconds: env.SESSION_TTL_SECONDS,
    }),
  });
};
