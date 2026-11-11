import { describe, expect, it } from 'vitest';
import { decodeBase32 } from '../../src/domain/auth/verify-totp/index.js';
import { encodeBase32, parseArgs, putSecureString } from './ssm-secret.mjs';

describe('ssm-secret encodeBase32', () => {
  it('encodes base32 that the backend decodes back to the same bytes', () => {
    const bytes = Buffer.from('12345678901234567890');

    expect(encodeBase32(bytes)).toBe('GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ');
    for (const n of [1, 7, 20, 33]) {
      const random = Buffer.from(Array.from({ length: n }, (_, i) => (i * 37 + 11) % 256));
      expect(decodeBase32(encodeBase32(random))).toEqual(random);
    }
  });
});

describe('ssm-secret parseArgs', () => {
  it('requires --profile unless --dry-run, and refuses unknown flags', () => {
    expect(parseArgs(['--dry-run'], '/n')).toEqual({
      dryRun: true,
      profile: undefined,
      region: 'ap-southeast-2',
      name: '/n',
    });
    expect(() => parseArgs([], '/n')).toThrow('--profile is required');
    expect(() => parseArgs(['--profile'], '/n')).toThrow('incomplete');
    expect(() => parseArgs(['--force'], '/n')).toThrow('unknown');
  });
});

describe('ssm-secret putSecureString', () => {
  it('passes the secret through a private temp file, never in argv, and removes the file', () => {
    const calls = [];
    const written = [];
    let removed = 0;
    const tempFiles = {
      write: (contents) => {
        written.push(contents);
        return { path: '/tmp/kristoff-ssm-x/input.json', remove: () => (removed += 1) };
      },
    };
    putSecureString(
      { name: '/kristoff/prod/totp-secret', profile: 'kristoff-prod', region: 'ap-southeast-2' },
      'SECRET',
      (...args) => calls.push(args),
      tempFiles,
    );

    const [[command, argv]] = calls;
    expect(command).toBe('aws');
    expect(argv).toEqual([
      'ssm',
      'put-parameter',
      '--cli-input-json',
      'file:///tmp/kristoff-ssm-x/input.json',
      '--profile',
      'kristoff-prod',
      '--region',
      'ap-southeast-2',
    ]);
    expect(argv.join(' ')).not.toContain('SECRET');
    expect(removed).toBe(1);
    expect(JSON.parse(written[0])).toEqual({
      Name: '/kristoff/prod/totp-secret',
      Value: 'SECRET',
      Type: 'SecureString',
      Overwrite: true,
    });
  });
});

describe('ssm-secret putSecureString failure', () => {
  it('removes the temp file even when the AWS CLI fails', () => {
    let removed = 0;
    const tempFiles = {
      write: () => ({ path: '/tmp/x/input.json', remove: () => (removed += 1) }),
    };
    const failing = () => {
      throw new Error('aws failed');
    };

    expect(() =>
      putSecureString({ name: '/n', profile: 'p', region: 'r' }, 'S', failing, tempFiles),
    ).toThrow('aws failed');
    expect(removed).toBe(1);
  });
});
