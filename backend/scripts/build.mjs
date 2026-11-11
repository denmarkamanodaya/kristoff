#!/usr/bin/env node
// build — one esbuild ESM bundle per Lambda, zipped deterministically to dist/<name>.zip (single
// entry index.mjs exporting `handler`, fixed timestamp and mode) plus dist/<name>.zip.sha256.
// Adapted from dexfs. Contract: docs/contract.md "Build artefacts". The Lambda names come from
// their one source, lambda-scope.json: every name must have src/handlers/<name>/index.ts and every
// handler directory (other than _shared and __tests__) must be a name, or the build fails.
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { zipOne } from './lib/zip.mjs';

const ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)), '..');
const HANDLERS = join(ROOT, 'src', 'handlers');
const DIST = join(ROOT, 'dist');
const ENTRY = 'index.mjs';
const BANNER =
  "import { createRequire } from 'node:module'; const require = createRequire(import.meta.url);";

const names = Object.keys(
  JSON.parse(readFileSync(join(ROOT, 'lambda-scope.json'), 'utf8')).functions,
);
const dirs = readdirSync(HANDLERS).filter((d) => !d.startsWith('_'));
const missing = names.filter((n) => !existsSync(join(HANDLERS, n, 'index.ts')));
const unknown = dirs.filter((d) => !names.includes(d));
if (missing.length > 0 || unknown.length > 0) {
  console.error(
    `build: missing handlers [${missing.join(', ')}], undeclared [${unknown.join(', ')}]`,
  );
  process.exit(1);
}

async function bundle(name) {
  const result = await build({
    entryPoints: [join(HANDLERS, name, 'index.ts')],
    bundle: true,
    write: false,
    platform: 'node',
    target: 'node24',
    format: 'esm',
    minify: true,
    legalComments: 'none',
    banner: { js: BANNER },
    logLevel: 'error',
  });
  const zip = zipOne(ENTRY, Buffer.from(result.outputFiles[0].contents));
  writeFileSync(join(DIST, `${name}.zip`), zip);
  const sha = createHash('sha256').update(zip).digest('hex');
  writeFileSync(join(DIST, `${name}.zip.sha256`), `${sha}  ${name}.zip\n`);
  return { name, bytes: zip.length, sha };
}

rmSync(DIST, { recursive: true, force: true });
mkdirSync(DIST, { recursive: true });
for (const name of names) {
  const out = await bundle(name);
  console.log(`build: dist/${out.name}.zip ${out.bytes} bytes sha256 ${out.sha}`);
}
console.log(`build: ${names.length} zips`);
