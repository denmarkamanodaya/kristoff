// Per-viewer display preferences. The ONLY module that touches localStorage (scripts/check.mjs
// enforces it), and it stores the theme choice only: key `kristoff.theme`, values `light|dark`.
// Contract: docs/contract.md, "Thread deletion + dark mode". Never store sign-in values here.

const THEME_KEY = 'kristoff.theme';
const THEMES = ['light', 'dark'];
const DEFAULT_THEME = 'light';

/**
 * Returns the saved theme.
 * @returns `'light'` or `'dark'`; `'light'` when nothing is saved, the value is unknown, or
 *   storage is blocked (private window, disabled site data).
 */
export function getTheme() {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    return THEMES.includes(saved) ? saved : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME; // recovery: storage unavailable, use the default
  }
}

/**
 * Saves the theme choice.
 * @param theme - `'light'` or `'dark'`; anything else saves `'light'`.
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
