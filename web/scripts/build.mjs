// Builds web/dist: the static site only. config.json is never copied; the deploy writes it
// from Terraform outputs (docs/contract.md, "Config").
import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { copyVendor } from './vendor.mjs';
import { checkSources } from './check.mjs';

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = resolve(webRoot, process.env.OUT_DIR ?? 'dist'); // OUT_DIR: a private dev build
const files = ['index.html', 'style.css', 'app.js'];
const dirs = ['services', 'ui'];

checkSources();
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
for (const f of files) cpSync(join(webRoot, f), join(dist, f));
for (const d of dirs) cpSync(join(webRoot, d), join(dist, d), { recursive: true });
copyVendor(join(dist, 'vendor'));
console.log(`build: wrote ${dist}`);
