// Kristoff web UI entry. Applies the theme first, then wires the gate and boots the app after an
// unlock; the desktop build has no gate (api.requiresUnlock is false, D-17) and boots at once. Every network call goes through services/api.js; the session lives in memory
// (services/session.js); UI modules live in ui/. Contract: docs/contract.md.
import * as api from './services/api.js';
import * as auth from './services/auth.js';
import * as prefs from './services/prefs.js';
import { hydrateIcons } from './services/icons.js';
import { $, $$, notify } from './ui/dom.js';
import { state } from './ui/state.js';
import { setRerender } from './ui/inline.js';
import { moreDocs, moreThreads, openThread, refreshDocs, refreshProjects, refreshThreads, renderUsage } from './ui/data.js';
import { newThread, renderAll, show } from './ui/views.js';
import { guard, handleError } from './ui/errors.js';
import { hideLoading, showLoading, waitForLaw } from './ui/loading.js';
import { renderComposer, renderModelPicker, setDraft } from './ui/composer.js';
import { openSettings, renderSettingsEntry } from './ui/settings.js';
import { shouldOnboard } from './ui/settings-flow.js';
import { openApp, setOnUnlock, showGate } from './ui/gate.js';
import { closeDrawer, toggleDrawer } from './ui/drawer.js';

// ---------- theme (applied first, so the page never paints in the wrong theme) ----------
function applyTheme(theme) {
  document.documentElement.dataset.theme = theme;
  $$('[data-theme-choice]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.themeChoice === theme)));
}
applyTheme(prefs.getTheme());
hydrateIcons();
$('.theme-toggle').addEventListener('click', (e) => {
  const b = e.target.closest('[data-theme-choice]');
  if (b) applyTheme(prefs.setTheme(b.dataset.themeChoice));
});

// ---------- wiring ----------
setRerender(renderAll);
$('#notice button').onclick = () => { $('#notice').hidden = true; };
$('#newThread').onclick = () => { closeDrawer(); newThread(); };
$$('.nav-item').forEach((a) => {
  a.onclick = () => { closeDrawer(); if (a.dataset.view === 'home') newThread(); else show('library'); };
});
$('#moreThreads').onclick = guard(moreThreads);
$('#moreDocs').onclick = guard(moreDocs);
$('#drawerBtn').onclick = toggleDrawer;
$('.suggestions').onclick = (e) => { const b = e.target.closest('button'); if (b) setDraft(b.dataset.q); };
document.addEventListener('keydown', (e) => {
  if (api.requiresUnlock && !auth.isUnlocked()) return;
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); newThread(); }
});

/** First load after the first unlock: config, projects, library, threads, usage. */
async function boot() {
  newThread();
  try {
    const cfg = await api.getConfig();
    state.model = cfg.model; state.models = cfg.models ?? []; state.maxAttachedDocs = cfg.maxAttachedDocs;
    state.features = cfg.features ?? {};
    renderModelPicker(); renderComposer(); renderSettingsEntry();
    await Promise.all([refreshProjects(), refreshDocs(), refreshThreads(), renderUsage()]);
    state.booted = true;
  } catch (err) {
    handleError(err);
  }
}

/** Reload after a re-unlock (session expiry): lists, usage and the open thread. */
async function resync() {
  try {
    await Promise.all([refreshProjects(), refreshDocs(), refreshThreads(), renderUsage()]);
    if (state.chat) await openThread(state.chat.id);
  } catch (err) {
    handleError(err);
  }
}

if (api.requiresUnlock) {
  setOnUnlock(() => (state.booted ? resync() : boot()));
  showGate();
} else {
  launchDesktop();
}

/** Desktop: the launch screen until the law index has warmed (or 60 s), then Home. */
async function launchDesktop() {
  showLoading();
  const { timedOut } = await waitForLaw(api, { onCount: showLoading });
  hideLoading();
  openApp();
  await boot();
  if (timedOut) notify('Statute search is still loading. Your first statute question may be slow.');
  // First launch with no saved key: open Settings in its "Connect your Claude account" mode (Phase 8).
  if (state.features?.aiReady === false) {
    const status = await api.getKeyStatus().catch(() => null);
    if (shouldOnboard(state.features, status)) openSettings({ onboarding: true });
  }
}
