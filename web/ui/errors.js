// Central error handling: one inline notice per failure plus the contract's resync action.
import { messageFor, notify } from './dom.js';
import { refreshDocs, refreshProjects, refreshThreads } from './data.js';
import { newThread } from './views.js';

/** Shows the error inline and runs the contract's client action for its code. */
export function handleError(err) {
  if (err?.name === 'AbortError') return;
  if (err?.code === 'UNAUTHENTICATED') return; // the session expiry listener already shows the gate
  console.error(err);
  notify(messageFor(err));
  if (err?.code === 'DOC_NOT_FOUND') refreshDocs().catch(handleError);
  if (err?.code === 'PROJECT_NOT_FOUND') Promise.all([refreshProjects(), refreshThreads()]).catch(handleError);
  if (err?.code === 'CHAT_NOT_FOUND') { refreshThreads().catch(handleError); newThread(); }
}

/** Wraps an async handler so a rejection goes to handleError. */
export function guard(fn) {
  return (...args) => Promise.resolve(fn(...args)).catch(handleError);
}
