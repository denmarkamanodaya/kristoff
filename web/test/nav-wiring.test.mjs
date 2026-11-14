// Sidebar wiring (Phase 8 regression, 2026-11-13): the Settings button shares the `nav-item` class
// with Home and Library but has no `data-view`. app.js once bound every `.nav-item` and sent anything
// that was not Home to Library, so clicking Settings opened the Library. These checks pin that the
// view handler only binds items that name a view, and that Settings names none.
// Run: `npm test` (node --test, no dependencies).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('the view handler binds only nav items that name a view', () => {
  const app = read('../app.js');

  assert.match(app, /\$\$\('\.nav-item\[data-view\]'\)\.forEach/);
  assert.doesNotMatch(app, /\$\$\('\.nav-item'\)/);
});

test('the Settings button is a nav item without a view, so only Settings handles its click', () => {
  const html = read('../index.html');
  const button = html.match(/<button[^>]*id="settingsBtn"[^>]*>/)?.[0] ?? '';

  assert.match(button, /class="nav-item settings-btn"/);
  assert.doesNotMatch(button, /data-view=/);
});
