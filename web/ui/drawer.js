// Mobile drawer (under 760px): the sidebar slides in over the page with a backdrop, so threads,
// projects, menus and the theme toggle stay reachable on a phone. Esc or the backdrop closes it.
import { $ } from './dom.js';
import { state } from './state.js';

const sidebar = $('.sidebar');
const btn = $('#drawerBtn');
const backdrop = $('#drawerBackdrop');

function apply() {
  sidebar.classList.toggle('open', state.drawerOpen);
  backdrop.hidden = !state.drawerOpen;
  btn.setAttribute('aria-expanded', String(state.drawerOpen));
}

/** Opens or closes the drawer; opening focuses the New Thread button. */
export function toggleDrawer() {
  state.drawerOpen = !state.drawerOpen;
  apply();
  if (state.drawerOpen) $('#newThread').focus(); else btn.focus();
}

/** Closes the drawer (no-op when closed). */
export function closeDrawer() {
  if (!state.drawerOpen) return;
  state.drawerOpen = false;
  apply();
}

backdrop.addEventListener('click', () => { closeDrawer(); btn.focus(); });
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && state.drawerOpen && !document.querySelector('.menu') && !state.confirm && !state.renaming) {
    closeDrawer(); btn.focus();
  }
});
sidebar.addEventListener('click', (e) => { if (e.target.closest('.thread-open')) closeDrawer(); });
