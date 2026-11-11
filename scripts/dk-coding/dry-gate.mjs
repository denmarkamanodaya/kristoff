#!/usr/bin/env node
/**
 * dry-gate — the dk-coding zero-clone gate, with the version-pair exemption built in.
 * Dependency-free Node ESM wrapper around jscpd (the repo's own copy; nothing is downloaded).
 *
 * Usage: node dry-gate.mjs [--root <pkgDir>] [--files a,b,...] [--json]
 *   --root <pkgDir>  package to scan (default: cwd). jscpd runs with cwd = pkgDir.
 *   --files a,b,...  report only clones touching these files (absolute, or relative to the cwd).
 *                    A clone also counts when it touches a file that is an exact copy of a listed file,
 *                    so editing v2 still reports the v1 <-> other-unit clone jscpd lists for that copy.
 *   --json           print {clones, exempt, scanned} as JSON on stdout instead of the text list.
 *
 * jscpd binary, first hit wins:
 *   1. $DRY_GATE_JSCPD (path to a jscpd executable; overrides discovery — used by the tests)
 *   2. <pkgDir>/node_modules/.bin/jscpd, then the same path in each parent directory
 *   3. jscpd on PATH
 * None found (or $DRY_GATE_JSCPD points at nothing): prints "dry-gate: jscpd not found — skipped"
 * to stderr and exits 0, so a hook never blocks on a missing tool.
 *
 * Config: <pkgDir>/.jscpd.json when present (jscpd also reads it itself); otherwise the defaults
 * min-tokens 35, min-lines 3, formats typescript/tsx/javascript/jsx/dart, ignoring tests
 * (**\/*.test.*, **\/*_test.dart, **\/test/**, **\/__tests__/**), node_modules, build, dist and
 * generated code (**\/generated/**, **\/*.g.dart). Reporters, output dir and exit code are always
 * overridden here; the JSON report is the only thing read.
 *
 * Version-pair exemption: a clone is dropped when both sides are versions of the SAME unit —
 * <unit>/v1.ts <-> <unit>/v2.ts, or the same path inside sibling version directories
 * (modules/x/v1/main.tf <-> modules/x/v2/main.tf). A copy across units is never exempt.
 *
 * Exit codes: 0 no clones (or jscpd missing), 1 clones remain (list: file:start-end <-> file:start-end),
 * 2 usage error or jscpd produced no report.
 */
import { spawnSync } from 'node:child_process';
import { accessSync, constants, existsSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const DEFAULTS = {
  minTokens: 35,
  minLines: 3,
  formats: ['typescript', 'tsx', 'javascript', 'jsx', 'dart'],
  ignore: ['**/*.test.*', '**/*_test.dart', '**/test/**', '**/__tests__/**', '**/node_modules/**', '**/build/**', '**/dist/**', '**/generated/**', '**/*.g.dart'],
};

function parseArgs(argv) {
  const opts = { root: canonical(process.cwd()), files: null, json: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--json') opts.json = true;
    else if (a === '--root' || a === '--files') {
      const v = argv[++i];
      if (!v || v.startsWith('--')) usageError(`${a} needs a value`);
      if (a === '--root') opts.root = canonical(resolve(v));
      else opts.files = v.split(',').filter(Boolean).map((f) => canonical(resolve(f)));
    } else usageError(`unknown argument ${a}`);
  }
  if (!existsSync(opts.root) || !statSync(opts.root).isDirectory()) usageError(`--root ${opts.root} is not a directory`);
  return opts;
}

function usageError(msg) {
  console.error(`dry-gate: ${msg}\nusage: dry-gate.mjs [--root <pkgDir>] [--files a,b,...] [--json]`);
  process.exit(2);
}

// macOS tmp paths come back from jscpd as /private/var/...; compare real paths on both sides.
function canonical(p) {
  try {
    return realpathSync(p);
  } catch {
    return p;
  }
}

function isExecutable(p) {
  try {
    accessSync(p, constants.X_OK);
    return statSync(p).isFile();
  } catch {
    return false;
  }
}

function findJscpd(root) {
  if (process.env.DRY_GATE_JSCPD !== undefined) return isExecutable(process.env.DRY_GATE_JSCPD) ? process.env.DRY_GATE_JSCPD : null;
  for (let dir = root; ; dir = dirname(dir)) {
    const p = join(dir, 'node_modules', '.bin', 'jscpd');
    if (isExecutable(p)) return p;
    if (dirname(dir) === dir) break;
  }
  for (const dir of (process.env.PATH ?? '').split(delimiter).filter(Boolean)) {
    if (isExecutable(join(dir, 'jscpd'))) return join(dir, 'jscpd');
  }
  return null;
}

function runJscpd(bin, root) {
  const out = mkdtempSync(join(tmpdir(), 'dry-gate-'));
  const hasConfig = existsSync(join(root, '.jscpd.json'));
  const args = hasConfig
    ? ['--config', '.jscpd.json']
    : ['--min-tokens', String(DEFAULTS.minTokens), '--min-lines', String(DEFAULTS.minLines), '--format', DEFAULTS.formats.join(','), '--ignore', DEFAULTS.ignore.join(','), '.'];
  // Only flags shared by jscpd 4 and 5; the exit status is ignored (a config `threshold` can fail it) and the report decides.
  args.push('--reporters', 'json', '--output', out, '--absolute', '--silent');
  try {
    const res = spawnSync(bin, args, { cwd: root, encoding: 'utf8', timeout: 120_000 });
    const report = join(out, 'jscpd-report.json');
    if (!existsSync(report)) {
      console.error(`dry-gate: jscpd produced no report (status ${res.status})\n${(res.stderr || res.stdout || String(res.error ?? '')).trim()}`);
      process.exit(2);
    }
    return { duplicates: JSON.parse(readFileSync(report, 'utf8')).duplicates ?? [], usedConfig: hasConfig };
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
}

// "<dir>/v3.ts" -> { key: "<dir>/v*.ts", n: 3 }; "<dir>/v2/main.tf" -> { key: "<dir>/v*/main.tf", n: 2 }.
// The LAST v<N> segment decides, so a/v1/x/v2.ts is keyed on its own unit (a/v1/x).
export function versionKey(file) {
  const parts = canonical(file).split('/');
  for (let i = parts.length - 1; i >= 0; i--) {
    const m = parts[i].match(/^v(\d+)(\.[A-Za-z0-9]+)?$/);
    if (!m || (i === parts.length - 1) !== (m[2] !== undefined)) continue; // file form needs an ext, dir form none
    return { key: [...parts.slice(0, i), `v*${m[2] ?? ''}`, ...parts.slice(i + 1)].join('/'), n: Number(m[1]) };
  }
  return null;
}

export function isVersionPair(a, b) {
  const ka = versionKey(a);
  const kb = versionKey(b);
  return ka !== null && kb !== null && ka.key === kb.key && ka.n !== kb.n;
}

// Union-find over fragments ("file:start-end") so a filtered run sees every copy of a fragment.
function fragmentGroups(dups) {
  const parent = new Map();
  const find = (x) => {
    if (!parent.has(x)) parent.set(x, x);
    while (parent.get(x) !== x) x = parent.get(x);
    return x;
  };
  const id = (f) => `${canonical(f.name)}:${f.start}-${f.end}`;
  for (const d of dups) parent.set(find(id(d.firstFile)), find(id(d.secondFile)));
  const members = new Map();
  for (const d of dups) {
    for (const f of [d.firstFile, d.secondFile]) {
      const r = find(id(f));
      if (!members.has(r)) members.set(r, new Set());
      members.get(r).add(canonical(f.name));
    }
  }
  return (d) => members.get(find(id(d.firstFile)));
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const bin = findJscpd(opts.root);
  if (!bin) {
    console.error('dry-gate: jscpd not found — skipped');
    process.exit(0);
  }
  const { duplicates } = runJscpd(bin, opts.root);
  const groupOf = fragmentGroups(duplicates);
  const wanted = opts.files ? new Set(opts.files) : null;
  const exempt = [];
  const clones = [];
  for (const d of duplicates) {
    if (isVersionPair(d.firstFile.name, d.secondFile.name)) { exempt.push(d); continue; }
    if (wanted && ![...groupOf(d)].some((f) => wanted.has(f))) continue;
    clones.push(d);
  }
  const show = (f) => `${relative(opts.root, canonical(f.name)) || f.name}:${f.start}-${f.end}`;
  if (opts.json) {
    const shape = (d) => ({ a: show(d.firstFile), b: show(d.secondFile), lines: d.lines, tokens: d.tokens });
    console.log(JSON.stringify({ clones: clones.map(shape), exempt: exempt.map(shape), scanned: opts.root }, null, 2));
  } else if (clones.length) {
    console.error(`dry-gate — ${clones.length} clone(s) (${exempt.length} same-unit version pair(s) exempt)\n` +
      clones.map((d) => `  ${show(d.firstFile)} <-> ${show(d.secondFile)}  (${d.lines} lines, ${d.tokens} tokens)`).join('\n') +
      '\nExtract the shared knowledge (see the dk-coding DRY rule) or, if the copies change for different reasons, say so in the PR.');
  } else {
    console.log(`dry-gate — OK (0 clones; ${exempt.length} same-unit version pair(s) exempt)`);
  }
  process.exit(clones.length ? 1 : 0);
}

if (isAbsolute(process.argv[1] ?? '') && canonical(process.argv[1]) === canonical(fileURLToPath(import.meta.url))) main();
