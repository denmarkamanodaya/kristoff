// Loads the deploy-time web config `/config.json` ({ apiBaseUrl, streamUrl }) once.
// Contract: docs/contract.md, "Config". The file is written by the deploy, never committed.

let pending = null;

/**
 * Returns the web config, fetching `/config.json` on the first call only.
 * @returns Promise of `{ apiBaseUrl, streamUrl }`, both without a trailing slash.
 * @throws Error when the file is missing or a URL is not a string.
 */
export function loadConfig() {
  pending ??= fetch('/config.json', { cache: 'no-store' })
    .then((r) => {
      if (!r.ok) throw new Error(`config.json: HTTP ${r.status}`);
      return r.json();
    })
    .then((c) => {
      if (typeof c?.apiBaseUrl !== 'string' || typeof c?.streamUrl !== 'string') {
        throw new Error('config.json: apiBaseUrl and streamUrl are required');
      }
      const trim = (u) => u.replace(/\/+$/, '');
      return Object.freeze({ apiBaseUrl: trim(c.apiBaseUrl), streamUrl: trim(c.streamUrl) });
    })
    .catch((err) => {
      pending = null; // allow a retry after a transient failure
      throw err;
    });
  return pending;
}
