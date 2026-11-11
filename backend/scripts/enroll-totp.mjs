#!/usr/bin/env node
// enroll-totp — makes the owner's TOTP secret (20 random bytes, RFC 6238 SHA-1) and stores it in
// SSM `/kristoff/prod/totp-secret` (docs/contract.md "Config"), then prints the setup key and the
// otpauth URI for "Enter a setup key" in an authenticator app. Re-running replaces the secret,
// so the old authenticator entry stops working.
// Usage: node scripts/enroll-totp.mjs --profile kristoff-prod [--region ap-southeast-2]
//        node scripts/enroll-totp.mjs --dry-run      (prints a key, writes nothing)
import { randomBytes } from 'node:crypto';
import { encodeBase32, parseArgs, putSecureString } from './lib/ssm-secret.mjs';

const options = parseArgs(process.argv.slice(2), '/kristoff/prod/totp-secret');
const secret = encodeBase32(randomBytes(20));
if (options.dryRun) console.log('dry run: nothing written to SSM');
else {
  putSecureString(options, secret);
  console.log(`wrote ${options.name} (SecureString) with profile ${options.profile}`);
}
console.log(`setup key: ${secret}`);
console.log(`otpauth://totp/Kristoff?secret=${secret}&issuer=Kristoff`);
