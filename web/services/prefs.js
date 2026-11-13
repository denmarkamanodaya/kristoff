// Per-viewer display preferences. The ONLY module that touches localStorage (scripts/check.mjs
// enforces it), and it stores the theme choice only: key `lexph.theme`, values `light|dark`. The
// pre-rename key `kristoff.theme` is read once, copied to the new key and removed (Phase 8 rename).
// Contract: docs/contract.md, "Thread deletion + dark mode". Never store sign-in values here.

const THEME_KEY = 'lexph.theme';
const OLD_THEME_KEY = 'kristoff.theme';

/** Reads the saved value, moving a pre-rename value to the new key the first time. */
function readSaved() {
  const saved = localStorage.getItem(THEME_KEY);
  if (saved !== null) return saved;
  const old = localStorage.getItem(OLD_THEME_KEY);
  if (old !== null) {
    localStorage.setItem(THEME_KEY, old);
    localStorage.removeItem(OLD_THEME_KEY);
  }
  return old;
}
const THEMES = ['light', 'dark'];
const DEFAULT_THEME = 'dark'; // owner ruling 2026-11-12: dark is the default

/**
 * Returns the saved theme.
 * @returns `'light'` or `'dark'`; `'dark'` when nothing is saved, the value is unknown, or
 *   storage is blocked (private window, disabled site data).
 */
export function getTheme() {
  try {
    const saved = readSaved();
    return THEMES.includes(saved) ? saved : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME; // recovery: storage unavailable, use the default
  }
}

/**
 * Saves the theme choice.
 * @param theme - `'light'` or `'dark'`; anything else saves `'dark'`.
 * @returns The theme that applies now. When storage is blocked it still applies for this page.
 */
export function setTheme(theme) {
  const value = THEMES.includes(theme) ? theme : DEFAULT_THEME;
  try {
    localStorage.setItem(THEME_KEY, value);
  } catch {
    // recovery: storage unavailable; the caller still applies `value` for this page load
  }
  return value;
}
