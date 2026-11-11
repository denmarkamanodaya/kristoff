// Shared by enroll-totp.mjs and gen-session-key.mjs: argument parsing and the one SSM write.
// The secret goes to the AWS CLI through a 0600 JSON file in a fresh private temp directory
// (--cli-input-json file://<path>), never in argv, so it does not appear in the process list or
// shell history; the directory is removed right after the call. (file:///dev/stdin was refused
// with EACCES when run through Claude Code's `!` prompt, 2026-11-12.)
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/** Real temp-file operations; tests inject a fake. */
const realTempFiles = {
  write(contents) {
    const dir = mkdtempSync(join(tmpdir(), 'kristoff-ssm-'));
    const path = join(dir, 'input.json');
    writeFileSync(path, contents, { mode: 0o600 });
    return { path, remove: () => rmSync(dir, { recursive: true, force: true }) };
  },
};

const BASE32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

/**
 * RFC 4648 base32 without padding (the form authenticator apps accept as a setup key).
 * @param bytes - Secret bytes.
 * @returns Upper-case base32 text.
 */
export function encodeBase32(bytes) {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += BASE32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += BASE32[(value << (5 - bits)) & 31];
  return out;
}

/**
 * Parses `--dry-run`, `--profile <p>`, `--region <r>` and `--name <parameter>`.
 * @param argv - `process.argv.slice(2)`.
 * @param defaultName - The SSM parameter name from docs/contract.md "Config".
 * @returns The options; throws on an unknown flag or a missing --profile without --dry-run.
 */
export function parseArgs(argv, defaultName) {
  const options = {
    dryRun: false,
    profile: undefined,
    region: 'ap-southeast-2',
    name: defaultName,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    if (flag === '--dry-run') options.dryRun = true;
    else if (['--profile', '--region', '--name'].includes(flag) && argv[i + 1]) {
      options[flag.slice(2)] = argv[i + 1];
      i += 1;
    } else throw new Error(`unknown or incomplete argument: ${flag}`);
  }
  if (!options.dryRun && !options.profile)
    throw new Error('--profile is required (or pass --dry-run)');
  return options;
}

/**
 * Writes the value as a SecureString, overwriting any previous value.
 * @param options - From parseArgs.
 * @param value - The secret.
 * @param run - Process runner (execFileSync; injectable for tests).
 * @param tempFiles - Writes the private input file and returns `{ path, remove }` (injectable for tests).
 */
export function putSecureString(options, value, run = execFileSync, tempFiles = realTempFiles) {
  const input = JSON.stringify({
    Name: options.name,
    Value: value,
    Type: 'SecureString',
    Overwrite: true,
  });
  const file = tempFiles.write(input);
  try {
    run(
      'aws',
      [
        'ssm',
        'put-parameter',
        '--cli-input-json',
        `file://${file.path}`,
        '--profile',
        options.profile,
        '--region',
        options.region,
      ],
      { stdio: ['ignore', 'ignore', 'inherit'] },
    );
  } finally {
    file.remove();
  }
}
