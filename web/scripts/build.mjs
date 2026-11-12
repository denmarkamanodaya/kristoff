// Builds the static site. Default target: web/dist (config.json is never copied; the deploy writes
// it from Terraform outputs, docs/contract.md "Config"). `--target desktop`: web/dist-desktop for
// the Tauri shell, with services/api-desktop.js served as services/api.js (so every ui/ import
// stays `./services/api.js`) and no services/config.js, since only api.js imports it and the
// desktop api.js never fetches /config.json (docs/desktop/phases/phase-1-core-storage.md §6).
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { copyVendor } from './vendor.mjs';
import { checkSources } from './check.mjs';
import { desktopIndex } from './desktop-index.mjs';

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetAt = process.argv.indexOf('--target');
const target = targetAt === -1 ? 'web' : process.argv[targetAt + 1];
if (target !== 'web' && target !== 'desktop') throw new Error(`build: unknown --target ${target}`);
const isDesktop = target === 'desktop';
const defaultOut = isDesktop ? 'dist-desktop' : 'dist';
const dist = resolve(webRoot, process.env.OUT_DIR ?? defaultOut); // OUT_DIR: a private dev build
const files = ['index.html', 'style.css', 'app.js'];
const dirs = ['services', 'ui'];

checkSources();
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
for (const f of files) cpSync(join(webRoot, f), join(dist, f));
for (const d of dirs) cpSync(join(webRoot, d), join(dist, d), { recursive: true });
const services = join(dist, 'services');
if (isDesktop) {
  cpSync(join(webRoot, 'services', 'api-desktop.js'), join(services, 'api.js'));
  rmSync(join(services, 'config.js'));
  writeFileSync(join(dist, 'index.html'), desktopIndex(readFileSync(join(webRoot, 'index.html'), 'utf8')));
}
rmSync(join(services, 'api-desktop.js')); // each build ships one api.js only
copyVendor(join(dist, 'vendor'));
console.log(`build: wrote ${dist} (target ${target})`);
