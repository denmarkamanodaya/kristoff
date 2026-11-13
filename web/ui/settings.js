// Settings panel (desktop only, Phase 8 bring-your-own-key): the Claude API key (masked hint,
// Change, Remove), the daily question limit, the data and privacy notices, and links. Opens from the
// sidebar gear, from any [data-open-settings] button, and by itself on first launch with no key.
// Plan: docs/desktop/phases/phase-8-lexph-byok.md §1. Rules live in ui/settings-flow.js.
import * as api from '../services/api.js';
import { hydrateIcons } from '../services/icons.js';
import { $, messageFor } from './dom.js';
import { state } from './state.js';
import { renderComposer } from './composer.js';
import { handleError } from './errors.js';
import { CAP_BOUNDS, KEY_CONSOLE_URL, OMNICORPUS_URL, checkDailyCap, checkKeyInput, keyLine } from './settings-flow.js';

const panel = $('#settings');
const keyForm = $('.key-form', panel);
const keyInput = $('#apiKey');
const keyMsg = $('.key-msg', panel);
const removeBtn = $('.key-remove', panel);
const capInput = $('#dailyCap');
const capMsg = $('.cap-msg', panel);
let opener = null;
let keyStatus = null;
let confirmingRemove = false;

hydrateIcons(panel);
capInput.min = String(CAP_BOUNDS.min);
capInput.max = String(CAP_BOUNDS.max);

/** Re-reads config so `features.aiReady` (and the composer gating) follows a key change. */
async function refreshFeatures() {
  const cfg = await api.getConfig();
  state.features = cfg.features ?? {};
  renderComposer();
}

function setBusy(form, busy) {
  form.querySelectorAll('button, input').forEach((el) => { el.disabled = busy; });
}

function renderKey() {
  $('.key-status', panel).textContent = keyLine(keyStatus);
  const has = keyStatus?.configured === true;
  $('.key-change', panel).textContent = has ? 'Change' : 'Add key';
  removeBtn.hidden = !has;
  removeBtn.textContent = confirmingRemove ? 'Confirm remove' : 'Remove';
}

function showKeyForm(open) {
  keyForm.hidden = !open;
  keyInput.value = '';
  keyMsg.textContent = '';
  if (open) keyInput.focus();
}

async function loadPanel() {
  const [status, settings] = await Promise.all([api.getKeyStatus(), api.getSettings()]);
  keyStatus = status;
  capInput.value = String(settings?.dailyQuestionCap ?? '');
  renderKey();
  if (!status?.configured) showKeyForm(true);
}

/**
 * Opens Settings.
 * @param opts - `{ onboarding }`: true shows the "Connect your Claude account" header and steps.
 */
export function openSettings({ onboarding = false } = {}) {
  opener = document.activeElement;
  confirmingRemove = false;
  $('#settingsTitle').textContent = onboarding ? 'Connect your Claude account' : 'Settings';
  $('.settings-steps', panel).hidden = !onboarding;
  keyMsg.textContent = ''; capMsg.textContent = '';
  showKeyForm(false);
  panel.hidden = false;
  $('.settings-close', panel).focus();
  loadPanel().catch((err) => { keyMsg.textContent = messageFor(err); });
}

/** Closes Settings and returns focus to whatever opened it. */
export function closeSettings() {
  panel.hidden = true;
  opener?.focus?.();
}

async function saveKey(e) {
  e.preventDefault();
  const checked = checkKeyInput(keyInput.value);
  if (checked.error) { keyMsg.textContent = checked.error; keyInput.focus(); return; }
  keyMsg.textContent = 'Checking the key with Anthropic…';
  setBusy(keyForm, true);
  try {
    keyStatus = await api.setApiKey(checked.key);
    showKeyForm(false);
    renderKey();
    keyMsg.textContent = 'Key saved. You can ask questions now.';
    await refreshFeatures();
  } catch (err) {
    keyMsg.textContent = err?.code === 'KEY_INVALID'
      ? 'Anthropic rejected that key. Check that you copied the whole key and that it is active.'
      : messageFor(err);
  } finally {
    setBusy(keyForm, false);
  }
}

async function removeKey() {
  if (!confirmingRemove) { confirmingRemove = true; renderKey(); return; }
  confirmingRemove = false;
  try {
    keyStatus = await api.removeApiKey() ?? { configured: false, hint: null };
    renderKey();
    keyMsg.textContent = 'Key removed. Add a key to ask questions again.';
    await refreshFeatures();
  } catch (err) {
    renderKey();
    keyMsg.textContent = messageFor(err);
  }
}

async function saveCap(e) {
  e.preventDefault();
  const checked = checkDailyCap(capInput.value, CAP_BOUNDS);
  if (checked.error) { capMsg.textContent = checked.error; capInput.focus(); return; }
  const form = e.currentTarget;
  setBusy(form, true);
  try {
    const s = await api.updateSettings({ dailyQuestionCap: checked.value });
    capInput.value = String(s?.dailyQuestionCap ?? checked.value);
    capMsg.textContent = 'Saved.';
    await refreshFeatures();
  } catch (err) {
    capMsg.textContent = messageFor(err);
  } finally {
    setBusy(form, false);
  }
}

/** Shows the sidebar gear in builds that have Settings (config reports `aiReady`). */
export function renderSettingsEntry() {
  $('#settingsBtn').hidden = state.features?.aiReady === undefined;
}

/** Sets the OmniCorpus credit line; its link opens through open_external. */
function renderCredit() {
  $('.credit', panel).innerHTML = 'Statute and decision data: <button type="button" class="link-btn" data-credit>OmniCorpus</button> (ODC-By), with credit to its author. Decisions show party names as published by the Supreme Court.';
}

// ---------- wiring ----------
renderCredit();
keyForm.addEventListener('submit', saveKey);
$('.key-change', panel).onclick = () => showKeyForm(keyForm.hidden);
$('.key-cancel', panel).onclick = () => showKeyForm(false);
removeBtn.onclick = removeKey;
$('.cap-form', panel).addEventListener('submit', saveCap);
$('.settings-close', panel).onclick = closeSettings;
$('#settingsBtn').onclick = () => openSettings();
panel.addEventListener('keydown', (e) => { if (e.key === 'Escape') { e.preventDefault(); closeSettings(); } });
panel.addEventListener('click', (e) => {
  if (e.target === panel) closeSettings();
  if (e.target.closest('[data-console]')) api.openExternal(KEY_CONSOLE_URL).catch(handleError);
  if (e.target.closest('[data-credit]')) api.openExternal(OMNICORPUS_URL).catch(handleError);
});
document.addEventListener('click', (e) => {
  if (e.target.closest('[data-open-settings]')) { e.preventDefault(); openSettings(); }
});
