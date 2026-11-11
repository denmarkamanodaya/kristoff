// In-memory session store. The token is never written to any browser storage or cookie
// (scripts/check.mjs enforces it), so every page load asks for a TOTP code
// (docs/contract.md, "Auth flow").

let token = null;
let expiryTimer = null;
const listeners = new Set();

/** Returns the current bearer token, or null when locked. */
export function getToken() {
  return token;
}

/**
 * Stores a fresh session and schedules its expiry.
 * @param session - `{ token, expiresAt }` from `POST /v1/auth/verify`; `expiresAt` is an ISO instant.
 */
export function setSession(session) {
  token = session.token;
  clearTimeout(expiryTimer);
  const ms = Date.parse(session.expiresAt) - Date.now();
  expiryTimer = setTimeout(() => expire(), Math.max(0, ms));
}

/** Drops the token and tells every listener the user must unlock again. */
export function expire() {
  const hadToken = token !== null;
  token = null;
  clearTimeout(expiryTimer);
  if (hadToken) listeners.forEach((fn) => fn());
}

/**
 * Subscribes to session expiry (timer, or an `UNAUTHENTICATED` response).
 * @param fn - Called with no arguments.
 * @returns An unsubscribe function.
 */
export function onExpired(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
