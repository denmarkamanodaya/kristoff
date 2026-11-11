// Syntax-checks every browser module with `node --check` and enforces the service-layer
// rules: only services/ may call fetch or XMLHttpRequest; only services/prefs.js may touch
// localStorage (theme choice), and it may not mention tokens or import the session; no file
// touches sessionStorage, cookies or IndexedDB, so session tokens are never stored.
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** Lists the browser JS sources (app.js plus services/*.js and ui/*.js). */
function sources() {
  const inDir = (d) => readdirSync(join(webRoot, d)).filter((f) => f.endsWith('.js')).map((f) => join(webRoot, d, f));
  return [join(webRoot, 'app.js'), ...inDir('services'), ...inDir('ui')];
}

/**
 * Throws when a source fails to parse or breaks the service-layer rules.
 * @returns Nothing; logs one line on success.
 */
export function checkSources() {
  const problems = [];
  for (const file of sources()) {
    const rel = relative(webRoot, file);
    execFileSync(process.execPath, ['--check', file], { stdio: 'inherit' });
    const code = readFileSync(file, 'utf8');
    const isService = rel.startsWith('services/');
    if (!isService && /\bfetch\s*\(|XMLHttpRequest/.test(code)) problems.push(`${rel}: network call outside services/`);
    const isPrefs = rel === 'services/prefs.js';
    if (/sessionStorage|document\.cookie|indexedDB/.test(code)) problems.push(`${rel}: web storage use`);
    if (!isPrefs && /localStorage/.test(code)) problems.push(`${rel}: localStorage outside services/prefs.js`);
    if (isPrefs && /token|session/i.test(code)) {
      problems.push(`${rel}: prefs must not handle session or token values`);
    }
    if (/\balert\s*\(|\bconfirm\s*\(|\bprompt\s*\(/.test(code)) problems.push(`${rel}: blocking dialog`);
  }
  if (problems.length) throw new Error(`check failed:\n${problems.join('\n')}`);
  console.log(`check: ${sources().length} files OK`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) checkSources();
