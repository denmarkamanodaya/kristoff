// SsmSecretSource: the SecretSource port on SSM SecureStrings (docs/contract.md "Config"). Each
// Lambda is given only the parameter names it may read; any other secret is refused.
import type { SecretName, SecretSource } from '../../../ports/secret-source.js';
import { appError } from '../../../shared/errors/app-error.js';
import { err } from '../../../shared/result/result.js';
import { cachedParameter, type ParameterTarget } from '../_shared/cached-parameter.js';

/**
 * Builds the SSM secret source.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Client and clock.
 * @param names - Parameter name per secret this Lambda may read.
 * @returns The SecretSource (60 s cache per secret).
 */
export const ssmSecretSource = (
  target: ParameterTarget,
  names: Readonly<Partial<Record<SecretName, string>>>,
): SecretSource => {
  const readers = new Map(
    Object.entries(names).map(([secret, name]) => [secret, cachedParameter(target, name)]),
  );
  return {
    get: (secret) => {
      const read = readers.get(secret);
      return read
        ? read()
        : Promise.resolve(err(appError('INTERNAL', `secret ${secret} not wired`)));
    },
  };
};
