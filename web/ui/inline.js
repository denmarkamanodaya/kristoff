// Inline delete confirmation and inline rename, shared by threads, projects and files. Never a
// blocking dialog. Each kind registers what "yes" and "save" do; renderers ask whether their row
// is confirming or renaming and print the markup from here.
import { $, esc } from './dom.js';
import { state } from './state.js';

const onConfirm = {}; // kind -> async (id) => void
const onRename = {}; // kind -> async (id, name) => void
let rerender = () => {};
let returnSel = null;

/** Sets the function that re-renders every view that can show an inline confirm or rename. */
export function setRerender(fn) { rerender = fn; }
/** Registers the delete action for a kind ('chat' | 'project' | 'doc'). */
export function registerConfirm(kind, fn) { onConfirm[kind] = fn; }
/** Registers the rename action for a kind. */
export function registerRename(kind, fn) { onRename[kind] = fn; }

/** True when the row `kind/id` at `where` shows the delete confirm. */
export const isConfirming = (kind, id, where) => state.confirm?.kind === kind && state.confirm.id === id && state.confirm.where === where;
/** True when the row `kind/id` at `where` shows the rename input. */
export const isRenaming = (kind, id, where) => state.renaming?.kind === kind && state.renaming.id === id && state.renaming.where === where;

/**
 * Shows the inline confirm on one row and focuses its Cancel button.
 * @param kind - 'chat' | 'project' | 'doc'.
 * @param id - The item id.
 * @param where - Which copy of the row ('side', 'head', 'lib', 'pop').
 * @param back - Selector of the element that gets focus back on cancel.
 */
export function askConfirm(kind, id, where, back) {
  state.confirm = { kind, id, where, busy: false };
  state.renaming = null;
  returnSel = back;
  rerender();
  $(`[data-confirm-for="${CSS.escape(`${kind}:${id}:${where}`)}"] .no`)?.focus();
}

/** Markup of the confirm box; `text` is plain text. */
export function confirmHtml(text) {
  const { kind, id, where, busy } = state.confirm;
  return `<div class="del-confirm" role="group" aria-label="Confirm delete" data-confirm-for="${esc(`${kind}:${id}:${where}`)}"><p>${esc(text)}</p>
    <div class="row"><button type="button" class="yes" data-confirm-yes ${busy ? 'disabled' : ''}>${busy ? 'Deleting…' : 'Delete'}</button>
    <button type="button" class="no" data-confirm-no ${busy ? 'disabled' : ''}>Cancel</button></div></div>`;
}

function backFocus() { if (returnSel) $(returnSel)?.focus(); returnSel = null; }

/** Closes the confirm without deleting (Cancel or Esc). */
export function cancelConfirm() {
  if (!state.confirm || state.confirm.busy) return;
  state.confirm = null;
  rerender();
  backFocus();
}

async function confirmYes() {
  const c = state.confirm;
  if (!c || c.busy) return;
  state.confirm = { ...c, busy: true };
  rerender();
  try {
    await onConfirm[c.kind](c.id);
  } finally {
    state.confirm = null;
    rerender();
  }
}

/** Shows the rename input on one row and focuses it with the text selected. */
export function startRename(kind, id, where, back) {
  state.renaming = { kind, id, where };
  state.confirm = null;
  returnSel = back;
  rerender();
  const input = $(`.rename-input[data-rename-for="${CSS.escape(`${kind}:${id}:${where}`)}"]`);
  input?.focus(); input?.select();
}

/** Markup of the rename input for the current rename row. */
export function renameHtml(value, label, max) {
  const { kind, id, where } = state.renaming;
  return `<input class="rename-input" type="text" maxlength="${max}" value="${esc(value)}" aria-label="${esc(label)}" data-rename-for="${esc(`${kind}:${id}:${where}`)}">`;
}

function endRename(focusBack) {
  state.renaming = null;
  rerender();
  if (focusBack) backFocus();
}

async function saveRename(input) {
  const r = state.renaming;
  const name = input.value.trim();
  if (!r || !name) { endRename(true); return; }
  input.disabled = true;
  try {
    await onRename[r.kind](r.id, name);
  } finally {
    endRename(true);
  }
}

document.addEventListener('click', (e) => {
  if (e.target.closest('[data-confirm-yes]')) confirmYes();
  else if (e.target.closest('[data-confirm-no]')) cancelConfirm();
});
document.addEventListener('keydown', (e) => {
  const input = e.target.closest?.('.rename-input');
  if (input && e.key === 'Enter') { e.preventDefault(); saveRename(input); return; }
  if (input && e.key === 'Escape') { e.preventDefault(); endRename(true); return; }
  if (e.key === 'Escape' && state.confirm) cancelConfirm();
});
document.addEventListener('focusout', (e) => {
  const input = e.target.closest?.('.rename-input');
  if (input && !input.disabled && state.renaming) setTimeout(() => { if (state.renaming && !input.disabled) endRename(false); }, 0);
});
