#!/usr/bin/env node
/**
 * check-versions — enforces the dk-coding side-by-side versioning layout.
 * Dependency-free Node ESM. Run it with cwd = the package root (the directory holding src/ or lib/).
 *
 * Usage: node scripts/check-versions.mjs [--config check-versions.json]
 *   --config <file>  JSON overriding the defaults below. If the flag is given and the file is missing
 *                    or not valid JSON, the script exits 2 (it never falls back to defaults silently).
 *
 * Config (JSON, defaults shown):
 * {
 *   "roots": ["src/core", "src/usecases", "src/services", "src/hooks", "src/handlers", "src/adapters", "src/domain", "src/contracts", "lib/core", "lib/services", "lib/data", "lib/platform"],
 *   "terraformModules": ["terraform/modules", "backend/terraform/modules"],
 *   "allowedImporters": ["src/composition", "src/app", "lib/app", "migrations"],
 *   "maxVersions": 3
 * }
 * Ports are never versioned, so no ports directory is a default root.
 *
 * A "unit" is any directory under a root that contains at least one v<N>.<ext> file.
 * Fails (exit 1) on:
 *  1. a file inside a unit that is not v<N>.<ext>, its test, the pointer (index.ts | current.dart), or README.md
 *  2. a v<N> file imported from anywhere except
 *     - its own test: v<N>.test.* or v<N>_test.dart in the unit, or a Dart test in the mirrored tree
 *       (test/<path>/v<N>_test.dart for lib/<path>/v<N>.dart); v10 never counts as v1's test;
 *     - its pointer (index.ts / current.dart in the unit);
 *     - a pointer test directly under __tests__/ or test/architecture/, named pointers.test.<ts|tsx|js|mjs>,
 *       pointers-<layer>.test.<ext>, pointers_test.dart or pointers_<layer>_test.dart;
 *     - an allowed importer root, or any migrations/ path
 *  3. v<N+1> importing v<N> of the same unit
 *  4. a v<N> file without a version marker (`@version` or `Version:`)
 *  5. a unit holding more than maxVersions v<N> files
 *  6. a Terraform modules/<name>/ directory holding .tf files outside v<N>/
 *
 * Import specifiers checked (comments stripped first):
 *  - TS/JS: relative specifiers ('./v1.js'), plus tsconfig.json `compilerOptions.paths` / `baseUrl` aliases
 *    read from the package root (no `extends` chain, no package.json `imports` field).
 *  - Dart: `import`/`export` of relative files ('v1.dart', '../x/v1.dart', `import 'v1.dart' as v1`) and
 *    `package:<this package>/...` (package name from pubspec.yaml, resolved under lib/). Other packages are skipped.
 * Exit codes: 0 OK, 1 violations, 2 usage/config error.
 */
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, relative, dirname, resolve, basename, extname, sep } from 'node:path';

const cwd = process.cwd();
const args = process.argv.slice(2);

function usageError(msg) {
  console.error(`check:versions — ${msg}`);
  process.exit(2);
}

function loadConfig() {
  const i = args.indexOf('--config');
  if (i === -1) return {};
  const p = args[i + 1];
  if (!p || p.startsWith('--')) usageError('--config needs a file path');
  const abs = resolve(cwd, p);
  if (!existsSync(abs)) usageError(`config file not found: ${p} (resolved to ${abs}); refusing to fall back to defaults`);
  try {
    return JSON.parse(readFileSync(abs, 'utf8'));
  } catch (e) {
    return usageError(`config file ${p} is not valid JSON: ${e.message}`);
  }
}

const defaults = {
  roots: ['src/core', 'src/usecases', 'src/services', 'src/hooks', 'src/handlers', 'src/adapters', 'src/domain', 'src/contracts', 'lib/core', 'lib/services', 'lib/data', 'lib/platform'],
  terraformModules: ['terraform/modules', 'backend/terraform/modules'],
  allowedImporters: ['src/composition', 'src/app', 'lib/app', 'migrations'],
  maxVersions: 3,
};
const cfg = { ...defaults, ...loadConfig() };
const errors = [];
const VERSION_FILE = /^v(\d+)\.(ts|tsx|js|mjs|dart)$/;
const TEST_FILE = /^v(\d+)(\.test\.(ts|tsx|js|mjs)|_test\.dart)$/;
const POINTER = new Set(['index.ts', 'index.js', 'index.mjs', 'current.dart']);
const POINTER_TEST = /(^|\/)(__tests__|test\/architecture)\/(pointers(-[a-z0-9-]+)?\.test\.(ts|tsx|js|mjs)|pointers(_[a-z0-9_]+)?_test\.dart)$/;
const SOURCE_EXT = new Set(['.ts', '.tsx', '.js', '.mjs', '.dart']);

const posix = (p) => p.split(sep).join('/');
const rel = (p) => posix(relative(cwd, p));
const isFile = (p) => existsSync(p) && statSync(p).isFile();

function walk(dir, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === '.terraform' || name.startsWith('.')) continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

// --- discover units -------------------------------------------------------
const units = new Map(); // unitDir -> { versions: Map<N, file> }
for (const root of cfg.roots) {
  for (const f of walk(resolve(cwd, root))) {
    const m = basename(f).match(VERSION_FILE);
    if (!m) continue;
    const u = units.get(dirname(f)) ?? { versions: new Map() };
    u.versions.set(Number(m[1]), f);
    units.set(dirname(f), u);
  }
}
for (const [dir, u] of units) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) continue;
    const ok = VERSION_FILE.test(name) || TEST_FILE.test(name) || POINTER.has(name) || name === 'README.md';
    if (!ok) errors.push(`[1 layout] ${rel(p)} — only v<N>, v<N> tests, the pointer and README.md belong in a unit directory`);
  }
  if (u.versions.size > cfg.maxVersions) errors.push(`[5 cap] ${rel(dir)} holds ${u.versions.size} versions (max ${cfg.maxVersions}); remove the oldest first`);
  for (const f of u.versions.values()) {
    if (!/@version\s+\d+|Version:\s*\d+/.test(readFileSync(f, 'utf8'))) errors.push(`[4 doc] ${rel(f)} lacks a version marker (@version N / Version: N)`);
  }
}

// --- specifier resolution -------------------------------------------------
// JSONC -> JSON: drops comments and trailing commas without touching string contents ("src/**/*.ts").
function parseJsonc(text) {
  let out = '';
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '"') {
      let j = i + 1;
      while (j < text.length && text[j] !== '"') j += text[j] === '\\' ? 2 : 1;
      out += text.slice(i, j + 1);
      i = j;
    } else if (c === '/' && text[i + 1] === '/') {
      while (i < text.length && text[i] !== '\n') i++;
      out += '\n';
    } else if (c === '/' && text[i + 1] === '*') {
      i = text.indexOf('*/', i + 2);
      if (i === -1) break;
      i++;
    } else out += c;
  }
  return JSON.parse(out.replace(/,(\s*[}\]])/g, '$1'));
}

function loadTsAliases() {
  const p = resolve(cwd, 'tsconfig.json');
  if (!isFile(p)) return null;
  let opts;
  try {
    opts = parseJsonc(readFileSync(p, 'utf8')).compilerOptions ?? {};
  } catch {
    return null; // an unreadable tsconfig only disables alias resolution
  }
  const baseDir = resolve(cwd, opts.baseUrl ?? '.');
  const paths = Object.entries(opts.paths ?? {}).map(([key, targets]) => {
    const star = key.indexOf('*');
    return { prefix: star === -1 ? key : key.slice(0, star), suffix: star === -1 ? '' : key.slice(star + 1), exact: star === -1, targets };
  });
  return { baseDir, paths, hasBaseUrl: opts.baseUrl !== undefined };
}
const tsAliases = loadTsAliases();
const dartPackage = (() => {
  const p = resolve(cwd, 'pubspec.yaml');
  return isFile(p) ? readFileSync(p, 'utf8').match(/^name:\s*([A-Za-z0-9_]+)/m)?.[1] ?? null : null;
})();

function resolveTsFile(base) {
  const noJs = base.replace(/\.(m?js|jsx)$/, '');
  const candidates = [base, noJs + '.ts', noJs + '.tsx', noJs + '.mjs', noJs + '.js', base + '.ts', base + '.tsx', base + '.js', base + '.mjs', join(base, 'index.ts'), join(base, 'index.tsx'), join(base, 'index.js')];
  return candidates.find(isFile) ?? null;
}

function resolveAlias(spec) {
  if (!tsAliases) return null;
  for (const a of tsAliases.paths) {
    const hit = a.exact ? spec === a.prefix : spec.startsWith(a.prefix) && spec.endsWith(a.suffix) && spec.length >= a.prefix.length + a.suffix.length;
    if (!hit) continue;
    const middle = a.exact ? '' : spec.slice(a.prefix.length, spec.length - a.suffix.length);
    for (const t of a.targets) {
      const found = resolveTsFile(resolve(tsAliases.baseDir, t.replace('*', middle)));
      if (found) return found;
    }
  }
  return tsAliases.hasBaseUrl ? resolveTsFile(resolve(tsAliases.baseDir, spec)) : null;
}

function resolveSpec(fromFile, spec) {
  if (extname(fromFile) === '.dart') {
    if (spec.startsWith('dart:')) return null;
    if (spec.startsWith('package:')) {
      const m = spec.match(/^package:([^/]+)\/(.+)$/);
      return m && m[1] === dartPackage ? (isFile(resolve(cwd, 'lib', m[2])) ? resolve(cwd, 'lib', m[2]) : null) : null;
    }
    const p = resolve(dirname(fromFile), spec);
    return isFile(p) ? p : null;
  }
  if (spec.startsWith('.')) return resolveTsFile(resolve(dirname(fromFile), spec));
  return resolveAlias(spec);
}

// test/<path>/v<N>_test.dart is the own test of lib/<path>/v<N>.dart (Dart tests live in a mirrored tree).
function isMirroredDartTest(fromFile, target) {
  const rf = rel(fromFile);
  const rt = rel(target);
  return rf.startsWith('test/') && rt.startsWith('lib/') && dirname(rf.slice(5)) === dirname(rt.slice(4));
}

// --- import scan ----------------------------------------------------------
const IMPORT_RE = /(?:import|export)\s[^'"]*?from\s*['"]([^'"]+)['"]|import\s*\(\s*['"]([^'"]+)['"]\s*\)|^\s*(?:import|export)\s+['"]([^'"]+)['"]/gm;
const allSources = [...new Set([...cfg.roots, ...cfg.allowedImporters, 'src', 'lib', 'test', 'tests', 'integration_test'].flatMap((r) => walk(resolve(cwd, r))))]
  .filter((f) => SOURCE_EXT.has(extname(f)));
// Comments are stripped before matching: a doc comment that quotes an import (an `@rollback` line such as
// "point index.ts back at ./v1.js") is text, not an import. Line comments keep `://` (URLs) intact.
const stripComments = (s) => s.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1');
for (const f of allSources) {
  const src = stripComments(readFileSync(f, 'utf8'));
  for (const m of src.matchAll(IMPORT_RE)) {
    const spec = m[1] ?? m[2] ?? m[3];
    if (!spec) continue;
    const target = resolveSpec(f, spec);
    if (!target) continue;
    const vm = basename(target).match(VERSION_FILE);
    if (!vm) continue;
    const fromName = basename(f);
    const sameUnit = dirname(f) === dirname(target);
    const testMatch = fromName.match(TEST_FILE);
    const isOwnTest = testMatch !== null && Number(testMatch[1]) === Number(vm[1]) && (sameUnit || isMirroredDartTest(f, target));
    const isPointer = sameUnit && POINTER.has(fromName);
    const isPointerTest = POINTER_TEST.test(posix(f));
    const isAllowed = isPointerTest || cfg.allowedImporters.some((r) => f.startsWith(resolve(cwd, r) + sep)) || /(^|\/)migrations\//.test(posix(f));
    if (sameUnit && VERSION_FILE.test(fromName)) { errors.push(`[3 cross-version] ${rel(f)} imports ${rel(target)} — v<N+1> never imports v<N>; move shared code to shared/ first`); continue; }
    if (!(isOwnTest || isPointer || isAllowed)) errors.push(`[2 importer] ${rel(f)} imports ${rel(target)} directly — import the pointer (${extname(f) === '.dart' ? 'current.dart' : 'index'}) instead`);
  }
}

// --- terraform modules ----------------------------------------------------
for (const root of cfg.terraformModules) {
  const abs = resolve(cwd, root);
  if (!existsSync(abs)) continue;
  for (const mod of readdirSync(abs)) {
    const mdir = join(abs, mod);
    if (!statSync(mdir).isDirectory()) continue;
    const tfHere = readdirSync(mdir).filter((n) => n.endsWith('.tf'));
    if (tfHere.length) errors.push(`[6 terraform] ${rel(mdir)} holds .tf files outside a v<N>/ directory: ${tfHere.join(', ')}`);
    const vdirs = readdirSync(mdir).filter((n) => /^v\d+$/.test(n));
    if (vdirs.length > cfg.maxVersions) errors.push(`[5 cap] ${rel(mdir)} holds ${vdirs.length} module versions (max ${cfg.maxVersions})`);
  }
}

if (errors.length) {
  console.error(`check:versions — ${errors.length} problem(s)\n` + errors.map((e) => '  ' + e).join('\n'));
  process.exit(1);
}
console.log(`check:versions — OK (${units.size} versioned unit(s) scanned, ${allSources.length} source file(s) import-checked)`);
