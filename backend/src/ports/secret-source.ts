// SecretSource port: server secrets by role (stack/security.md). Contract: docs/contract.md
// "Config" (SSM SecureStrings). Values never reach logs or the wire.
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';

/** The secrets the backend uses, by role. */
export type SecretName = 'totp-secret' | 'session-key' | 'anthropic-key';

/** Reads secrets. */
export interface SecretSource {
  /** The secret's value; an error when it is missing or empty (fail closed). */
  get(name: SecretName): Promise<Result<string, AppError>>;
}
