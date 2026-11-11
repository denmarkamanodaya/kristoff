// Bundles one TypeScript entry with esbuild into a throwaway directory and imports it, so build
// scripts can read the Zod contracts (the one wire source) without a TS loader.
import { rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { build } from 'esbuild';

/** The backend package root. */
export const ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)), '..', '..');

/**
 * Bundles `entry` (absolute path, or source text with `contents`) and imports the result.
 * @param name - Scratch directory name under dist/.
 * @param entry - `{ path }` of a .ts file, or `{ contents }` of TS source resolved from src/.
 * @returns The module namespace.
 */
export async function importTs(name, entry) {
  const dir = join(ROOT, 'dist', `.${name}`);
  const outfile = join(dir, 'entry.mjs');
  rmSync(dir, { recursive: true, force: true });
  await build({
    ...(entry.path
      ? { entryPoints: [entry.path] }
      : { stdin: { contents: entry.contents, resolveDir: join(ROOT, 'src'), loader: 'ts' } }),
    outfile,
    bundle: true,
    // npm packages stay imports (resolved from backend/node_modules), so CommonJS SDKs load natively.
    packages: 'external',
    platform: 'node',
    format: 'esm',
    logLevel: 'error',
  });
  try {
    return await import(pathToFileURL(outfile).href);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
