#!/usr/bin/env node
// gen-session-key — makes the session HMAC key (32 random bytes, base64) and stores it in SSM
// `/kristoff/prod/session-hmac-key` (docs/contract.md "Config"). Rotating it signs every open
// session out. The key is printed only in --dry-run.
// Usage: node scripts/gen-session-key.mjs --profile kristoff-prod [--region ap-southeast-2]
//        node scripts/gen-session-key.mjs --dry-run
import { randomBytes } from 'node:crypto';
import { parseArgs, putSecureString } from './lib/ssm-secret.mjs';

const options = parseArgs(process.argv.slice(2), '/kristoff/prod/session-hmac-key');
const key = randomBytes(32).toString('base64');
if (options.dryRun) console.log(`dry run: nothing written to SSM\nkey: ${key}`);
else {
  putSecureString(options, key);
  console.log(`wrote ${options.name} (SecureString) with profile ${options.profile}`);
}
