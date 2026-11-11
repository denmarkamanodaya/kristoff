// Popover menu (role="menu") used by every ⋯ button: arrow keys, Home/End, Enter/Space, Esc,
// ArrowRight/Left for one submenu level, click outside to close, and focus returns to the
// button that opened it. Items: { label, icon, danger, checked, disabled, action, submenu }
// or { separator: true }.
import { icon } from '../services/icons.js';
import { esc } from './dom.js';

let root = null; // { el, anchor, child }

function itemsHtml(items) {
  return items.map((it, i) => {
    if (it.separator) return '<div class="menu-sep" role="separator"></div>';
    const attrs = [
      `data-i="${i}"`, 'role="menuitem"', 'tabindex="-1"',
      it.submenu ? 'aria-haspopup="menu" aria-expanded="false"' : '',
      it.disabled ? 'aria-disabled="true"' : '',
    ].join(' ');
    return `<button type="button" class="menu-item${it.danger ? ' danger' : ''}" ${attrs}>${icon(it.icon)}<span>${esc(it.label)}</span>${it.checked ? icon('check', 'menu-check') : ''}${it.submenu ? icon('chevron-right', 'menu-sub') : ''}</button>`;
  }).join('');
}

function place(el, rect, side) {
  const w = el.offsetWidth; const h = el.offsetHeight;
  let left = side ? rect.right + 4 : rect.right - w;
  if (side && left + w > innerWidth - 8) left = rect.left - w - 4;
  left = Math.max(8, Math.min(left, innerWidth - w - 8));
  let top = side ? rect.top - 4 : rect.bottom + 4;
  if (top + h > innerHeight - 8) top = Math.max(8, (side ? rect.bottom : rect.top) - h - 4);
  el.style.left = `${left}px`; el.style.top = `${top}px`;
}

const menuItems = (el) => [...el.querySelectorAll('[role="menuitem"]')];
function focusAt(el, i) { const list = menuItems(el); if (list.length) list[(i + list.length) % list.length].focus(); }

function build(items, label, rect, side) {
  const el = document.createElement('div');
  el.className = 'menu'; el.setAttribute('role', 'menu');
  if (label) el.setAttribute('aria-label', label);
  el.innerHTML = itemsHtml(items);
  document.body.append(el);
  place(el, rect, side);
  el.addEventListener('keydown', (e) => onKey(e, el, items));
  el.addEventListener('click', (e) => { const b = e.target.closest('[role="menuitem"]'); if (b) activate(b, items); });
  return el;
}

function closeChild() {
  if (!root?.child) return;
  root.child.el.remove();
  root.child.parentItem.setAttribute('aria-expanded', 'false');
  root.child.parentItem.focus();
  root.child = null;
}

function openChild(itemEl, it) {
  if (root.child) closeChild();
  const el = build(it.submenu, it.label, itemEl.getBoundingClientRect(), true);
  itemEl.setAttribute('aria-expanded', 'true');
  root.child = { el, parentItem: itemEl };
  focusAt(el, 0);
}

function activate(b, items) {
  const it = items[Number(b.dataset.i)];
  if (!it || it.disabled) return;
  if (it.submenu) { openChild(b, it); return; }
  closeMenu(false);
  it.action?.();
}

function onKey(e, el, items) {
  const list = menuItems(el);
  const i = list.indexOf(document.activeElement);
  const isChild = root?.child?.el === el;
  const keys = {
    ArrowDown: () => focusAt(el, i + 1), ArrowUp: () => focusAt(el, i - 1),
    Home: () => focusAt(el, 0), End: () => focusAt(el, list.length - 1),
    Enter: () => list[i] && activate(list[i], items), ' ': () => list[i] && activate(list[i], items),
    ArrowRight: () => { const it = items[Number(list[i]?.dataset.i)]; if (it?.submenu) openChild(list[i], it); },
    ArrowLeft: () => isChild && closeChild(),
    Escape: () => (isChild ? closeChild() : closeMenu(true)),
    Tab: () => closeMenu(false),
  };
  if (!keys[e.key]) return;
  if (e.key !== 'Tab') e.preventDefault();
  e.stopPropagation();
  keys[e.key]();
}

/**
 * Opens a menu under `anchor` (the ⋯ button) and focuses its first item.
 * @param anchor - The button that opened it; gets aria-expanded and focus back on Esc.
 * @param items - Menu items (see the file header).
 * @param label - Accessible name of the menu.
 */
export function openMenu(anchor, items, label) {
  const reopen = root?.anchor === anchor;
  closeMenu(false);
  if (reopen) return; // a second click on the same ⋯ closes it
  const el = build(items, label, anchor.getBoundingClientRect(), false);
  anchor.setAttribute('aria-expanded', 'true');
  root = { el, anchor, child: null };
  focusAt(el, 0);
}

/**
 * Closes the open menu (and its submenu).
 * @param returnFocus - True to move focus back to the button that opened it.
 */
export function closeMenu(returnFocus) {
  if (!root) return;
  const { el, anchor, child } = root;
  child?.el.remove();
  el.remove();
  anchor.setAttribute('aria-expanded', 'false');
  root = null;
  if (returnFocus && anchor.isConnected) anchor.focus();
}

/** True when the event target is inside an open menu. */
export const inMenu = (target) => Boolean(target.closest?.('.menu'));

document.addEventListener('mousedown', (e) => { if (root && !inMenu(e.target) && !root.anchor.contains(e.target)) closeMenu(false); });
addEventListener('resize', () => closeMenu(false));
document.addEventListener('scroll', (e) => { if (root && !inMenu(e.target)) closeMenu(false); }, true);
