import type { SSMClient } from '@aws-sdk/client-ssm';
import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { ssmFlagReader } from './v1.js';

const reader = (...replies: unknown[]) => {
  const { client, calls } = stubClient(...replies);
  let now = 0;
  const flags = ssmFlagReader(
    { client: client as unknown as SSMClient, now: () => (now += 61_000) },
    '/kristoff/prod/ai-enabled',
  );
  return { flags, calls };
};

describe('ssm-flag-reader v1', () => {
  it('reads exactly "true" and "false"', async () => {
    const { flags, calls } = reader(
      { Parameter: { Value: 'true' } },
      { Parameter: { Value: 'false' } },
    );

    expect([await flags.isAiEnabled(), await flags.isAiEnabled()]).toEqual([
      { ok: true, value: true },
      { ok: true, value: false },
    ]);
    expect(calls[0]?.input).toEqual({ Name: '/kristoff/prod/ai-enabled', WithDecryption: true });
  });

  it('fails closed on any other value and on a failed read', async () => {
    const { flags } = reader({ Parameter: { Value: 'yes' } }, sdkError('ThrottlingException'));

    expect(await flags.isAiEnabled()).toMatchObject({ ok: false, error: { code: 'INTERNAL' } });
    expect(await flags.isAiEnabled()).toMatchObject({
      ok: false,
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });
});
