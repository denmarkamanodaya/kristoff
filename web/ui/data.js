// Loads and updates server data through services/api.js, then re-renders what changed.
import * as api from '../services/api.js';
import { $, appendNew, fmt, upsert } from './dom.js';
import { state } from './state.js';
import { renderAll, show } from './views.js';
import { handleError } from './errors.js';

const POLL_MS = 3000;
const POLL_MAX = 200; // about 10 minutes of "processing" before the client stops asking
let pollTimer = null;
let polls = 0;

/** Reloads the first page of threads. */
export async function refreshThreads() {
  const r = await api.listChats();
  state.chats = r.items; state.chatsCursor = r.nextCursor;
  renderAll();
}
/** Appends the next page of threads. */
export async function moreThreads() {
  const r = await api.listChats(state.chatsCursor);
  appendNew(state.chats, r.items); state.chatsCursor = r.nextCursor;
  renderAll();
}
/** Reloads the project list (name order, ≤ 50). */
export async function refreshProjects() {
  state.projects = (await api.listProjects()).items;
  renderAll();
}
/** Reloads the first page of files and restarts the "processing" poll when needed. */
export async function refreshDocs() {
  const r = await api.listDocs();
  state.docs = r.items; state.docsCursor = r.nextCursor; state.docsLoaded = true;
  renderAll();
  watchProcessing();
}
/** Appends the next page of files. */
export async function moreDocs() {
  const r = await api.listDocs(state.docsCursor);
  appendNew(state.docs, r.items); state.docsCursor = r.nextCursor;
  renderAll();
}

/** Polls the first docs page while any loaded doc is "processing" (large-PDF split). */
export function watchProcessing() {
  if (pollTimer || !state.docs.some((d) => d.status === 'processing')) { if (!pollTimer) polls = 0; return; }
  pollTimer = setTimeout(async () => {
    pollTimer = null;
    polls += 1;
    if (polls > POLL_MAX) return;
    try {
      const { items } = await api.listDocs();
      for (const d of items) if (state.docs.some((x) => x.id === d.id)) upsert(state.docs, d);
      renderAll();
      watchProcessing();
    } catch (err) {
      handleError(err);
    }
  }, POLL_MS);
}

/** Opens a thread with its messages. */
export async function openThread(id) {
  const { chat, messages } = await api.getChat(id);
  state.chat = { ...chat, messages };
  show('thread');
}

/**
 * Applies a partial chat update (title, docIds, pinned, projectId) and merges the result.
 * @param id - Chat id.
 * @param patch - The fields to change.
 */
export async function patchChat(id, patch) {
  const { chat } = await api.updateChat(id, patch);
  const i = state.chats.findIndex((c) => c.id === id);
  if (i >= 0) state.chats[i] = { ...state.chats[i], ...chat };
  if (state.chat?.id === id) state.chat = { ...state.chat, ...chat };
  renderAll();
  return chat;
}

/** Renders today's and total token usage in the sidebar footer. */
export async function renderUsage() {
  const { today: t, total: u } = await api.getUsage();
  $('#usage').innerHTML = `Today · in ${fmt(t.input)} · out ${fmt(t.output)}<br>Total · in ${fmt(u.input)} · out ${fmt(u.output)}<br>cached ${fmt(u.cacheRead)} · cache write ${fmt(u.cacheWrite)}`;
}
