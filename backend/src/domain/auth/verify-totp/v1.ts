// RFC 6238 TOTP check with node:crypto (deterministic, so allowed in the domain). Contract:
// docs/contract.md "Auth flow" step 5: SHA-1, 6 digits, 30 s step, window ±1 step.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { createHmac, timingSafeEqual } from 'node:crypto';

/** Seconds per TOTP time step. */
export const TOTP_STEP_SECONDS = 30;
/** Digits in a code. */
const DIGITS = 6;
/** Steps accepted either side of the current one. */
const WINDOW = 1;
const BASE32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

/**
 * Decodes an RFC 4648 base32 secret (case-insensitive, `=` padding and spaces ignored).
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param text - Base32 text.
 * @returns The secret bytes, or `undefined` when a character is not base32 or the text is empty.
 */
export const decodeBase32 = (text: string): Buffer | undefined => {
  const clean = text.replace(/[\s=]/g, '').toUpperCase();
  if (clean.length === 0) return undefined;
  let bits = 0;
  let value = 0;
  const out: number[] = [];
  for (const char of clean) {
    const index = BASE32.indexOf(char);
    if (index < 0) return undefined;
    value = (value << 5) | index;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      out.push((value >>> bits) & 0xff);
    }
  }
  return Buffer.from(out);
};

/**
 * Computes the HOTP code (RFC 4226) for one counter.
 * @param secret - Shared secret bytes.
 * @param counter - Time step (non-negative integer).
 * @param digits - Code length (6 for login; 8 for the RFC test vectors).
 * @returns The zero-padded code.
 */
export const hotp = (secret: Buffer, counter: number, digits: number = DIGITS): string => {
  const message = Buffer.alloc(8);
  message.writeBigUInt64BE(BigInt(counter));
  const mac = createHmac('sha1', secret).update(message).digest();
  const offset = (mac[mac.length - 1] ?? 0) & 0x0f;
  const binary = mac.readUInt32BE(offset) & 0x7fffffff;
  return String(binary % 10 ** digits).padStart(digits, '0');
};

/**
 * The time step that contains an instant.
 * @param nowMs - Epoch milliseconds.
 * @returns `floor(seconds / 30)`.
 */
export const stepAt = (nowMs: number): number => Math.floor(nowMs / 1000 / TOTP_STEP_SECONDS);

/**
 * Finds the step within ±1 of now whose code equals `code`, comparing in constant time.
 * @param secret - Shared secret bytes.
 * @param code - Six digits from the user.
 * @param nowMs - Epoch milliseconds from the injected clock.
 * @returns The matched step, or `undefined` when no step in the window matches.
 */
export const matchTotpStep = (secret: Buffer, code: string, nowMs: number): number | undefined => {
  if (!/^\d{6}$/.test(code)) return undefined;
  const given = Buffer.from(code);
  const current = stepAt(nowMs);
  let matched: number | undefined;
  for (let step = current - WINDOW; step <= current + WINDOW; step += 1) {
    const isSame = timingSafeEqual(Buffer.from(hotp(secret, step)), given);
    if (isSame && matched === undefined) matched = step;
  }
  return matched;
};
