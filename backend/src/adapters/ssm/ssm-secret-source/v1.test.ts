import type { SSMClient } from '@aws-sdk/client-ssm';
import { describe, expect, it } from 'vitest';
import { sdkError, stubClient } from '../../../../test/support/sdk-stub.js';
import { ssmSecretSource } from './v1.js';

describe('ssm-secret-source v1', () => {
  it('reads the mapped parameter with decryption and caches it for 60 s', async () => {
    let now = 0;
    const { client, calls } = stubClient(
      { Parameter: { Value: 's1' } },
      { Parameter: { Value: 's2' } },
    );
    const source = ssmSecretSource(
      { client: client as unknown as SSMClient, now: () => now },
      { 'session-key': '/kristoff/prod/session-hmac-key' },
    );

    const first = await source.get('session-key');
    now = 59_999;
    const cached = await source.get('session-key');
    now = 60_000;
    const fresh = await source.get('session-key');

    expect([first, cached, fresh]).toEqual([
      { ok: true, value: 's1' },
      { ok: true, value: 's1' },
      { ok: true, value: 's2' },
    ]);
    expect(calls[0]).toEqual({
      command: 'GetParameterCommand',
      input: { Name: '/kristoff/prod/session-hmac-key', WithDecryption: true },
    });
  });

  it('fails closed: empty value and a missing parameter are INTERNAL, a failed call is UPSTREAM_UNAVAILABLE', async () => {
    const { client } = stubClient(
      { Parameter: { Value: ' ' } },
      sdkError('ParameterNotFound'),
      sdkError('ThrottlingException'),
    );
    const source = ssmSecretSource(
      { client: client as unknown as SSMClient, now: () => 0 },
      { 'totp-secret': '/x' },
    );

    expect(await source.get('totp-secret')).toMatchObject({ error: { code: 'INTERNAL' } });
    expect(await source.get('totp-secret')).toMatchObject({ error: { code: 'INTERNAL' } });
    expect(await source.get('totp-secret')).toMatchObject({
      error: { code: 'UPSTREAM_UNAVAILABLE' },
    });
  });

  it('refuses a secret this Lambda was not wired for, without calling SSM', async () => {
    const { client, calls } = stubClient();
    const source = ssmSecretSource({ client: client as unknown as SSMClient, now: () => 0 }, {});

    expect(await source.get('anthropic-key')).toMatchObject({ error: { code: 'INTERNAL' } });
    expect(calls).toEqual([]);
  });
});
