// Small DOM and formatting helpers shared by every UI module. Imports nothing from ui/, so any
// module may use these at evaluation time.
import { keyErrorText } from './settings-flow.js';

/** Returns the first match of `s` under `el`. */
export const $ = (s, el = document) => el.querySelector(s);
/** Returns every match of `s` under `el` as an array. */
export const $$ = (s, el = document) => [...el.querySelectorAll(s)];
/** Escapes text for HTML element and attribute content. */
export const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
/** Formats an integer with locale grouping (null → 0). */
export const fmt = (n) => (n || 0).toLocaleString();
/** Formats a byte count as KB or MB. */
export const kb = (b) => (b > 1048576 ? `${(b / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);
/** Replaces the item with the same id, or puts it first. */
export const upsert = (list, item) => { const i = list.findIndex((x) => x.id === item.id); if (i >= 0) list[i] = item; else list.unshift(item); };
/** Appends items whose id is not in the list yet. */
export const appendNew = (list, items) => { const seen = new Set(list.map((x) => x.id)); list.push(...items.filter((x) => !seen.has(x.id))); };
/** Shortens a title for chips: at most `max` characters, with an ellipsis. */
export const shortTitle = (t, max = 28) => (String(t).length > max ? `${String(t).slice(0, max - 1).trimEnd()}…` : String(t));
/** Formats a page range: "p. 12" or "pp. 12–14"; empty when no page. */
export const pageLabel = (page, endPage) => (page ? (endPage && endPage !== page ? `pp. ${page}–${endPage}` : `p. ${page}`) : '');

/**
 * Shows a dismissible inline notice above the current view.
 * @param text - Plain text (never HTML).
 */
export function notify(text) {
  const n = $('#notice');
  $('span', n).textContent = text;
  n.hidden = false;
}

/** Turns an ApiError into the sentence the user sees, per the contract's client actions. */
export function messageFor(err) {
  const ref = err?.requestId ? ` (ref ${err.requestId})` : '';
  switch (err?.code) {
    case 'OTP_INVALID': return 'That code is not right. Check your authenticator app and try again.';
    case 'OTP_REPLAYED': return 'That code was already used. Wait for the next code, then enter it.';
    case 'OTP_LOCKED': {
      const m = Math.max(1, Math.ceil((err.retryAfter ?? 900) / 60));
      return `Too many wrong codes. Try again in ${m} minute${m > 1 ? 's' : ''}.`;
    }
    case 'UNAUTHENTICATED': return 'Your session ended. Enter a new code.';
    case 'VALIDATION_FAILED': return `The request was not accepted: ${err.message}`;
    case 'DOC_NOT_FOUND': return 'A file could not be found. The library was refreshed.';
    case 'CHAT_NOT_FOUND': return 'That thread no longer exists. The thread list was refreshed.';
    case 'PROJECT_NOT_FOUND': return 'That project no longer exists. The project list was refreshed.';
    case 'PROJECT_LIMIT_REACHED': return 'You have reached the project limit. Delete a project to make a new one.';
    case 'UPLOAD_INVALID': return 'The file was rejected: it is not a valid PDF, or its size did not match.';
    case 'DAILY_CAP_REACHED': return "You reached today's question limit. Try again tomorrow.";
    case 'AI_DISABLED': return 'Answers are switched off for now. Try again later.';
    case 'KEY_MISSING':
    case 'KEY_INVALID': return keyErrorText(err.code);
    case 'UPSTREAM_UNAVAILABLE':
    case 'INTERNAL': return `Something went wrong on the server. Try again in a moment.${ref}`;
    default: return (err?.message || 'Something went wrong.') + ref;
  }
}
