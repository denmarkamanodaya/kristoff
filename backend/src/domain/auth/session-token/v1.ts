// Session token sign/verify (docs/contract.md "Auth flow" step 4):
// base64url(payloadJson) + "." + base64url(HMAC-SHA256(key, payloadPart)), MAC compared in
// constant time. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { createHmac, timingSafeEqual } from 'node:crypto';

/** What a session token carries. Times are epoch seconds. */
export interface SessionPayload {
  readonly sub: string;
  readonly iat: number;
  readonly exp: number;
}

/** Longest token accepted, in characters (bounds the work done on garbage input). */
const MAX_TOKEN = 1024;

const mac = (key: Buffer, payloadPart: string): Buffer =>
  createHmac('sha256', key).update(payloadPart, 'utf8').digest();

/**
 * Signs a session payload.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param key - HMAC key bytes (at least 32).
 * @param payload - Subject and epoch-second times.
 * @returns The token `payloadPart.macPart`.
 */
export const signSession = (key: Buffer, payload: SessionPayload): string => {
  const payloadPart = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64url');
  return `${payloadPart}.${mac(key, payloadPart).toString('base64url')}`;
};

const isPayload = (value: unknown): value is SessionPayload => {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.sub === 'string' &&
    Number.isInteger(v.iat) &&
    Number.isInteger(v.exp) &&
    Object.keys(v).length === 3
  );
};

const parsePayload = (payloadPart: string): SessionPayload | undefined => {
  try {
    const value: unknown = JSON.parse(Buffer.from(payloadPart, 'base64url').toString('utf8'));
    return isPayload(value) ? value : undefined;
  } catch {
    return undefined;
  }
};

/**
 * Verifies a token's MAC (constant time) and expiry.
 * @param key - HMAC key bytes.
 * @param token - Token from the `Authorization` header.
 * @param nowSeconds - Epoch seconds from the injected clock.
 * @returns The payload, or `undefined` when the token is malformed, forged or expired.
 */
export const verifySessionToken = (
  key: Buffer,
  token: string,
  nowSeconds: number,
): SessionPayload | undefined => {
  if (token.length > MAX_TOKEN) return undefined;
  const [payloadPart, macPart, ...rest] = token.split('.');
  if (payloadPart === undefined || macPart === undefined || rest.length > 0) return undefined;
  const given = Buffer.from(macPart, 'base64url');
  const expected = mac(key, payloadPart);
  if (given.length !== expected.length || !timingSafeEqual(given, expected)) return undefined;
  const payload = parsePayload(payloadPart);
  return payload !== undefined && payload.exp > nowSeconds ? payload : undefined;
};
