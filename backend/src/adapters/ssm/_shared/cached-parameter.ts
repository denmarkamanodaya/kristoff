// SSM adapter plumbing (core/dry.md placement: adapters/<service>/_shared): reads one parameter
// with a 60 s cache. A missing or empty value is an error (stack/errors-and-resilience.md
// "Fail closed"); errors are never cached. Contract: docs/contract.md "Config".
import { GetParameterCommand, type SSMClient } from '@aws-sdk/client-ssm';
import { appError, type AppError } from '../../../shared/errors/app-error.js';
import { err, ok, type Result } from '../../../shared/result/result.js';
import { errorTypeOf } from '../../../shared/errors/vendor-error.js';

/** How long a read value is reused, in milliseconds. */
const PARAMETER_TTL_MS = 60_000;

/** What a cached reader needs. */
export interface ParameterTarget {
  readonly client: Pick<SSMClient, 'send'>;
  /** Epoch milliseconds. */
  readonly now: () => number;
}

/**
 * Builds a cached reader for one parameter name.
 * @param target - Client and clock.
 * @param name - Parameter name (from env, set by Terraform).
 * @returns A reader: the value, `INTERNAL` when missing or empty, `UPSTREAM_UNAVAILABLE` on a
 *   failed call.
 */
export const cachedParameter = (target: ParameterTarget, name: string) => {
  let cached: { value: string; at: number } | undefined;
  return async (): Promise<Result<string, AppError>> => {
    const now = target.now();
    if (cached !== undefined && now - cached.at < PARAMETER_TTL_MS) return ok(cached.value);
    try {
      const out = await target.client.send(
        new GetParameterCommand({ Name: name, WithDecryption: true }),
      );
      const value = out.Parameter?.Value ?? '';
      if (value.trim() === '')
        return err(appError('INTERNAL', 'parameter is empty', { operation: 'ssm.GetParameter' }));
      cached = { value, at: now };
      return ok(value);
    } catch (error) {
      const type = errorTypeOf(error);
      const code = type === 'ParameterNotFound' ? 'INTERNAL' : 'UPSTREAM_UNAVAILABLE';
      return err(
        appError(code, 'parameter read failed', {
          operation: 'ssm.GetParameter',
          'error.type': type,
        }),
      );
    }
  };
};
