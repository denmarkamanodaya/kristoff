// Sidebar: Pinned → Projects (collapsible, each with its threads) → Recents, with a ⋯ menu per
// thread (Pin, Add to project ▸, Rename, Delete) and per project (Rename, Delete).
// Contract: docs/contract.md "v2" (pinned, projectId, projects CRUD).
import * as api from '../services/api.js';
import { icon } from '../services/icons.js';
import { $, esc, notify } from './dom.js';
import { NAME_MAX, state } from './state.js';
import { openMenu } from './menu.js';
import {
  askConfirm, confirmHtml, isConfirming, isRenaming, registerConfirm, registerRename, renameHtml, startRename,
} from './inline.js';
import { openThread, patchChat } from './data.js';
import { newThread, renderAll } from './views.js';
import { guard, handleError } from './errors.js';

const sel = (attr, id) => `[${attr}="${CSS.escape(id)}"]`;

function threadHtml(c) {
  const active = c.id === state.chat?.id ? ' active' : '';
  if (isConfirming('chat', c.id, 'side')) {
    return `<li class="confirming${active}" data-id="${esc(c.id)}">${confirmHtml("Delete this thread? This can't be undone.")}</li>`;
  }
  if (isRenaming('chat', c.id, 'side')) return `<li class="thread-item renaming${active}">${renameHtml(c.title, 'Thread name', NAME_MAX.chat)}</li>`;
  const pin = c.pinned ? icon('pin', 'pin-mark') : '';
  return `<li class="thread-item${active}" data-id="${esc(c.id)}" title="${esc(c.title)}">
    <button type="button" class="thread-open"${active ? ' aria-current="page"' : ''}>${pin}<span>${esc(c.title)}</span></button>
    <button type="button" class="row-more" data-chat-menu="${esc(c.id)}" aria-haspopup="menu" aria-expanded="false" aria-label="Thread options: ${esc(c.title)}">${icon('more-horizontal')}</button></li>`;
}

function projectHtml(p, chats) {
  const open = !state.collapsed.has(p.id);
  if (isConfirming('project', p.id, 'side')) {
    return `<li class="project confirming">${confirmHtml('Delete this project? Its threads move to Recents.')}</li>`;
  }
  const row = isRenaming('project', p.id, 'side')
    ? `<div class="project-row renaming">${renameHtml(p.name, 'Project name', NAME_MAX.project)}</div>`
    : `<div class="project-row">
        <button type="button" class="project-toggle" data-project-toggle="${esc(p.id)}" aria-expanded="${open}">${icon('chevron-right', 'chev')}${icon('folder')}<span>${esc(p.name)}</span><span class="count">${chats.length}</span></button>
        <button type="button" class="row-more" data-project-menu="${esc(p.id)}" aria-haspopup="menu" aria-expanded="false" aria-label="Project options: ${esc(p.name)}">${icon('more-horizontal')}</button></div>`;
  const list = open ? `<ul class="thread-list nested">${chats.map(threadHtml).join('') || '<li class="side-empty">No threads yet</li>'}</ul>` : '';
  return `<li class="project${open ? ' open' : ''}">${row}${list}</li>`;
}

function newProjectHtml() {
  return state.newProject
    ? '<div class="new-project"><input class="new-project-input" type="text" maxlength="80" placeholder="Project name" aria-label="New project name"></div>'
    : '';
}

/** Renders the three sidebar sections from state. */
export function renderSidebar() {
  const known = new Set(state.projects.map((p) => p.id));
  const pinned = state.chats.filter((c) => c.pinned);
  const inProject = (p) => state.chats.filter((c) => !c.pinned && c.projectId === p.id);
  const recents = state.chats.filter((c) => !c.pinned && !known.has(c.projectId));
  const pinnedSec = pinned.length
    ? `<section class="side-sec" aria-label="Pinned"><h2 class="side-label">Pinned</h2><ul class="thread-list">${pinned.map(threadHtml).join('')}</ul></section>` : '';
  const projects = `<section class="side-sec" aria-label="Projects"><h2 class="side-label">Projects
      <button type="button" class="side-add" data-new-project aria-label="New project">${icon('plus')}</button></h2>
      ${newProjectHtml()}<ul class="project-list">${state.projects.map((p) => projectHtml(p, inProject(p))).join('')}</ul>
      ${state.projects.length || state.newProject ? '' : '<p class="side-empty">Group threads into projects.</p>'}</section>`;
  const recentSec = `<section class="side-sec" aria-label="Recents"><h2 class="side-label">Recents</h2><ul class="thread-list">${recents.map(threadHtml).join('') || '<li class="side-empty">No threads yet</li>'}</ul></section>`;
  $('#sideThreads').innerHTML = pinnedSec + projects + recentSec;
  $('#moreThreads').hidden = !state.chatsCursor;
}

// ---------- thread menu ----------
function projectSubmenu(c) {
  const items = state.projects.map((p) => ({
    label: p.name, icon: 'folder', checked: c.projectId === p.id,
    action: guard(() => patchChat(c.id, { projectId: c.projectId === p.id ? null : p.id })),
  }));
  if (c.projectId) items.push({ label: 'Remove from project', icon: 'x', action: guard(() => patchChat(c.id, { projectId: null })) });
  if (items.length) items.push({ separator: true });
  items.push({ label: 'New project…', icon: 'plus', action: () => startNewProject(c.id) });
  return items;
}

function openThreadMenu(btn, c) {
  const back = sel('data-chat-menu', c.id);
  openMenu(btn, [
    { label: c.pinned ? 'Unpin' : 'Pin', icon: 'pin', action: guard(() => patchChat(c.id, { pinned: !c.pinned })) },
    { label: 'Add to project', icon: 'folder', submenu: projectSubmenu(c) },
    { label: 'Rename', icon: 'pencil', action: () => startRename('chat', c.id, 'side', back) },
    { separator: true },
    { label: 'Delete', icon: 'trash-2', danger: true, action: () => askDeleteChat(c.id, 'side', back) },
  ], `Options for ${c.title}`);
}

/** Opens the delete confirm for a thread, unless its answer is still streaming. */
export function askDeleteChat(id, where, back) {
  if (state.busy && state.chat?.id === id) { notify('Wait for the answer to finish, then delete the thread.'); return; }
  askConfirm('chat', id, where, back);
}

// ---------- projects ----------
function startNewProject(forChatId) {
  state.newProject = { forChatId };
  renderSidebar();
  $('.new-project-input')?.focus();
}

async function createProject(name, forChatId) {
  const { project } = await api.createProject(name);
  state.projects = [...state.projects, project].sort((a, b) => a.name.localeCompare(b.name));
  state.collapsed.delete(project.id);
  if (forChatId) await patchChat(forChatId, { projectId: project.id });
  else renderAll();
}

function openProjectMenu(btn, p) {
  const back = sel('data-project-menu', p.id);
  openMenu(btn, [
    { label: 'Rename', icon: 'pencil', action: () => startRename('project', p.id, 'side', back) },
    { separator: true },
    { label: 'Delete', icon: 'trash-2', danger: true, action: () => askConfirm('project', p.id, 'side', back) },
  ], `Options for project ${p.name}`);
}

registerConfirm('chat', async (id) => {
  try {
    await api.deleteChat(id);
  } catch (err) {
    if (err?.code !== 'CHAT_NOT_FOUND') { handleError(err); return; } // already gone counts as deleted
  }
  state.chats = state.chats.filter((c) => c.id !== id);
  if (state.chat?.id === id) newThread();
});
registerConfirm('project', async (id) => {
  try {
    await api.deleteProject(id);
  } catch (err) {
    if (err?.code !== 'PROJECT_NOT_FOUND') { handleError(err); return; }
  }
  state.projects = state.projects.filter((p) => p.id !== id);
  state.chats = state.chats.map((c) => (c.projectId === id ? { ...c, projectId: null } : c));
});
registerRename('chat', (id, title) => patchChat(id, { title }).catch(handleError));
registerRename('project', async (id, name) => {
  try {
    const { project } = await api.renameProject(id, name);
    state.projects = state.projects.map((p) => (p.id === id ? project : p)).sort((a, b) => a.name.localeCompare(b.name));
  } catch (err) {
    handleError(err);
  }
});

// ---------- wiring ----------
$('#sideThreads').addEventListener('click', (e) => {
  const menuBtn = e.target.closest('[data-chat-menu]');
  if (menuBtn) { const c = state.chats.find((x) => x.id === menuBtn.dataset.chatMenu); if (c) openThreadMenu(menuBtn, c); return; }
  const pBtn = e.target.closest('[data-project-menu]');
  if (pBtn) { const p = state.projects.find((x) => x.id === pBtn.dataset.projectMenu); if (p) openProjectMenu(pBtn, p); return; }
  const toggle = e.target.closest('[data-project-toggle]');
  if (toggle) {
    const id = toggle.dataset.projectToggle;
    if (state.collapsed.has(id)) state.collapsed.delete(id); else state.collapsed.add(id);
    renderSidebar();
    $(sel('data-project-toggle', id))?.focus();
    return;
  }
  if (e.target.closest('[data-new-project]')) { startNewProject(null); return; }
  const open = e.target.closest('.thread-open');
  if (open) guard(openThread)(open.closest('li[data-id]').dataset.id);
});
$('#sideThreads').addEventListener('keydown', (e) => {
  const input = e.target.closest('.new-project-input');
  if (!input) return;
  if (e.key === 'Escape') { state.newProject = null; renderSidebar(); $('[data-new-project]')?.focus(); return; }
  if (e.key !== 'Enter' || !input.value.trim()) return;
  e.preventDefault();
  const { forChatId } = state.newProject;
  state.newProject = null;
  guard(createProject)(input.value.trim(), forChatId);
});
