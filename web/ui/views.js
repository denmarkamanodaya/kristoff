// View switching (home, thread, library) and the one "render everything that depends on state".
import { $, $$ } from './dom.js';
import { state } from './state.js';
import { renderSidebar } from './sidebar.js';
import { renderLibrary } from './library.js';
import { mountComposer, renderComposer } from './composer.js';
import { renderThread, renderThreadHead } from './thread.js';

/** Re-renders every state-driven part of the page except the thread turns. */
export function renderAll() {
  renderSidebar();
  renderLibrary();
  renderComposer();
  if (state.view === 'thread' && state.chat) renderThreadHead();
}

/**
 * Switches the main view and moves the shared composer into it.
 * @param view - 'home' | 'thread' | 'library'.
 */
export function show(view) {
  state.view = view;
  ['home', 'thread', 'library'].forEach((v) => { $(`#${v}`).hidden = v !== view; });
  $$('.nav-item').forEach((a) => a.classList.toggle('active', a.dataset.view === view));
  if (view === 'home') mountComposer($('#homeSlot'), 'home');
  if (view === 'thread') { mountComposer($('#threadSlot'), 'thread'); renderThread(); }
  renderAll();
}

/** Starts a new thread: clears the open chat and shows home. */
export function newThread() {
  state.chat = null; state.pendingDocIds = []; state.pendingStatutes = true;
  show('home');
}
