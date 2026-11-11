// TOTP unlock: exchanges a 6-digit code for an in-memory session token.
// Contract: docs/contract.md, "Auth flow" and `POST /v1/auth/verify`.
import { verifyCode } from './api.js';
import { expire, getToken, onExpired, setSession } from './session.js';

/** True when a 6-character string is all digits. */
export const isCode = (s) => /^\d{6}$/.test(s);

/**
 * Verifies a TOTP code and stores the session in memory.
 * @param code - Exactly 6 digits.
 * @returns Promise that resolves once the session is stored.
 * @throws ApiError (OTP_INVALID, OTP_REPLAYED, OTP_LOCKED, VALIDATION_FAILED, ...).
 */
export async function verify(code) {
  const session = await verifyCode(code);
  setSession(session);
}

/** True while a session token is held. */
export const isUnlocked = () => getToken() !== null;

export { expire as lock, onExpired };
