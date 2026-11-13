// Pure rules for the Settings panel and the bring-your-own-key flow (Phase 8,
// docs/desktop/phases/phase-8-lexph-byok.md §1). No DOM, so test/settings-flow.test.mjs can load it;
// ui/settings.js renders the panel. Contract: docs/contract.md, "Desktop IPC" (key and settings commands).

/** Where users create a Claude API key (opened through open_external). */
export const KEY_CONSOLE_URL = 'https://console.anthropic.com/settings/keys';

/** OmniCorpus dataset page, for the ODC-By credit (opened through open_external). */
export const OMNICORPUS_URL = 'https://huggingface.co/datasets/mongramosjr/philippine-omnicorpus';

/** Daily question limit range (contract `update_settings`: int 1..10000). */
export const CAP_BOUNDS = Object.freeze({ min: 1, max: 10000 });

/** Client-side bound on a pasted key; the shell validates the real format and checks it live. */
const KEY_MAX_CHARS = 256;

/**
 * Text for the saved-key line.
 * @param status - `{ configured, hint }` from `get_key_status`; `hint` is the key's last 4 characters.
 * @returns "sk-ant-…ab12 ✓", "Saved ✓" when there is no hint, or "No key saved".
 */
export function keyLine(status) {
  if (!status?.configured) return 'No key saved';
  return status.hint ? `sk-ant-…${status.hint} ✓` : 'Saved ✓';
}

/**
 * Checks a pasted key before it is sent, so an obvious slip gets an instant message.
 * @param raw - The input value.
 * @returns `{ key }` (trimmed) to send, or `{ error }` to show.
 */
export function checkKeyInput(raw) {
  const key = String(raw ?? '').trim();
  if (!key) return { error: 'Paste your Claude API key first.' };
  if (!key.startsWith('sk-ant-')) return { error: 'Claude API keys start with "sk-ant-". Check that you copied the whole key.' };
  if (key.length > KEY_MAX_CHARS || /\s/.test(key)) return { error: 'That does not look like a Claude API key. Copy it again from the Anthropic console.' };
  return { key };
}

/**
 * Checks the daily question limit field.
 * @param raw - The input value.
 * @param bounds - `{ min, max }` inclusive.
 * @returns `{ value }` (integer) or `{ error }`.
 */
export function checkDailyCap(raw, { min, max }) {
  const text = String(raw ?? '').trim();
  const value = Number(text);
  if (!/^\d+$/.test(text) || !Number.isInteger(value) || value < min || value > max) {
    return { error: `Enter a whole number from ${min} to ${max}.` };
  }
  return { value };
}

/**
 * Whether Settings opens by itself after the launch screen: only when no key is saved at all.
 * A saved but rejected key shows the composer prompt instead.
 * @param features - `get_config().features`.
 * @param status - `{ configured }` from `get_key_status`, or null when it could not be read.
 * @returns True to open Settings in its "Connect your Claude account" mode.
 */
export const shouldOnboard = (features, status) => features?.aiReady === false && status?.configured === false;

/** Whether asking is blocked until a key works (`aiReady` absent means a build without BYOK: not blocked). */
export const askBlocked = (features) => features?.aiReady === false;

/** Error codes that mean "fix your key in Settings". */
const KEY_CODES = new Set(['KEY_MISSING', 'KEY_INVALID']);

/** True when an error should offer the "open Settings" button. */
export const isKeyError = (err) => KEY_CODES.has(err?.code);

/**
 * Friendly text for the key error codes.
 * @param code - `KEY_MISSING` or `KEY_INVALID`.
 * @returns The message, or null for any other code.
 */
export function keyErrorText(code) {
  if (code === 'KEY_MISSING') return 'Add your Claude API key in Settings to ask questions.';
  if (code === 'KEY_INVALID') return 'Your Claude API key was rejected. Update it in Settings.';
  return null;
}
