// Desktop launch screen (owner request after D-17): wait until the law index has warmed (vectors
// and model loaded) so the first question is not slow and the UI is never half ready. Readiness
// comes from the `law-ready` event or `get_config().features.lawReady`, polled every 500 ms for at
// most 60 s; after that the app continues anyway. The web build never shows it (no lawReady).
import { $ } from './dom.js';

const POLL_MS = 500;
const TIMEOUT_MS = 60_000;

/** "Loading 10,870 Philippine laws…", or without the number when it is not known. */
export const loadingText = (lawCount) =>
  Number.isFinite(lawCount)
    ? `Loading ${lawCount.toLocaleString('en-US')} Philippine laws…`
    : 'Loading Philippine laws…';

/**
 * Waits until the law index is ready or the time is up.
 * @param api - `{ getConfig, onLawReady }` (services/api.js).
 * @param opts - `{ onCount(lawCount), pollMs, timeoutMs, sleep(ms), now() }`; the last two are for
 *   tests.
 * @returns `{ timedOut }`: false once ready (event, or `lawReady !== false` in the config).
 */
export async function waitForLaw(api, opts = {}) {
  const { onCount = () => {}, pollMs = POLL_MS, timeoutMs = TIMEOUT_MS } = opts;
  const sleep = opts.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
  const now = opts.now ?? (() => Date.now());
  let ready = false;
  let wake = () => {};
  const unlisten = await api.onLawReady(() => { ready = true; wake(); });
  const started = now();
  try {
    while (!ready) {
      const features = await api.getConfig().then((c) => c?.features ?? {}, () => null);
      if (features) {
        onCount(features.lawCount);
        if (features.lawReady !== false) return { timedOut: false };
      }
      if (now() - started >= timeoutMs) return { timedOut: true };
      await Promise.race([sleep(pollMs), new Promise((r) => { wake = r; })]);
    }
    return { timedOut: false };
  } finally {
    unlisten();
  }
}

/** Shows the launch screen with the law count when known. */
export function showLoading(lawCount) {
  $('#loadingText').textContent = loadingText(lawCount);
  $('#loading').hidden = false;
}

/** Hides the launch screen. */
export function hideLoading() {
  $('#loading').hidden = true;
}
