// TOTP gate: a full-screen 6-digit prompt. Nothing else renders or loads until a code is
// verified; the session token stays in memory (services/session.js). The desktop build has no
// #gate element (decision D-17, stripped by scripts/build.mjs), so nothing here wires up there.
import * as auth from '../services/auth.js';
import { $, messageFor } from './dom.js';

const gate = $('#gate');
const otp = $('#otp');
const gateMsg = $('#gateMsg');
const gateBtn = $('.gate-btn');
let verifying = false;
let lockTimer = null;
let onUnlock = async () => {};

function setGateEnabled(on) { otp.disabled = !on; gateBtn.disabled = !on; }

/**
 * Shows the gate and hides the app.
 * @param msg - Plain text shown under the input.
 */
export function showGate(msg = '') {
  gate.hidden = false;
  $('#app').hidden = true;
  otp.value = '';
  gateMsg.textContent = msg;
  if (!lockTimer) setGateEnabled(true);
  setTimeout(() => otp.focus(), 0);
}

/** Hides the gate and shows the app (after an unlock, or at once on desktop, D-17). */
export function openApp() {
  if (gate) gate.hidden = true;
  $('#app').hidden = false;
}

function lockGate(seconds) {
  setGateEnabled(false);
  clearTimeout(lockTimer);
  lockTimer = setTimeout(() => { lockTimer = null; setGateEnabled(true); gateMsg.textContent = ''; otp.focus(); }, (seconds ?? 900) * 1000);
}

async function submitCode() {
  const code = otp.value;
  if (verifying || lockTimer) return;
  if (!auth.isCode(code)) { gateMsg.textContent = 'Enter all 6 digits.'; return; }
  verifying = true;
  setGateEnabled(false);
  gateMsg.textContent = '';
  try {
    await auth.verify(code);
    openApp();
    await onUnlock();
  } catch (err) {
    otp.value = '';
    gateMsg.textContent = messageFor(err);
    setGateEnabled(true);
    if (err?.code === 'OTP_LOCKED') lockGate(err.retryAfter);
    else otp.focus();
  } finally {
    verifying = false;
  }
}

/** Sets what runs after a successful unlock (first boot or a resync). */
export function setOnUnlock(fn) { onUnlock = fn; }

if (gate) {
  otp.addEventListener('input', () => {
    const digits = otp.value.replace(/\D/g, '').slice(0, 6);
    if (digits !== otp.value) otp.value = digits;
    if (digits.length === 6) submitCode();
  });
  $('#gateForm').onsubmit = (e) => { e.preventDefault(); submitCode(); };
  auth.onExpired(() => showGate('Your session ended. Enter a new code.'));
}
