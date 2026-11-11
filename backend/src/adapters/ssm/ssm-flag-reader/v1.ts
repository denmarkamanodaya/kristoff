// SsmFlagReader: the FlagReader port on `/kristoff/prod/ai-enabled` (docs/contract.md "Config",
// stack/feature-flags.md). Only the exact strings `true` and `false` are valid; anything else
// fails closed. 60 s cache.
import type { FlagReader } from '../../../ports/flag-reader.js';
import { appError } from '../../../shared/errors/app-error.js';
import { err, ok } from '../../../shared/result/result.js';
import { cachedParameter, type ParameterTarget } from '../_shared/cached-parameter.js';

/**
 * Builds the SSM flag reader.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param target - Client and clock.
 * @param aiEnabledName - The kill-switch parameter name (env `SSM_AI_ENABLED`).
 * @returns The FlagReader.
 */
export const ssmFlagReader = (target: ParameterTarget, aiEnabledName: string): FlagReader => {
  const read = cachedParameter(target, aiEnabledName);
  return {
    isAiEnabled: async () => {
      const value = await read();
      if (!value.ok) return value;
      if (value.value === 'true') return ok(true);
      if (value.value === 'false') return ok(false);
      return err(appError('INTERNAL', 'kill switch is not true or false'));
    },
  };
};
