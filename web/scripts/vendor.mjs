// Copies the two browser libraries (marked, DOMPurify) from node_modules into a vendor
// directory, so the static site never depends on a CDN or a server route.
// Usage: node scripts/vendor.mjs <outDir>   (outDir is relative to web/; default "vendor")
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const webRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** Published file name -> source path inside web/node_modules. */
export const vendorFiles = {
  'marked.js': 'node_modules/marked/lib/marked.umd.js',
  'purify.js': 'node_modules/dompurify/dist/purify.min.js',
};

/**
 * Copies every vendor file into `outDir`.
 * @param outDir - Absolute target directory.
 */
export function copyVendor(outDir) {
  mkdirSync(outDir, { recursive: true });
  for (const [name, src] of Object.entries(vendorFiles)) {
    copyFileSync(join(webRoot, src), join(outDir, name));
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const out = resolve(webRoot, process.argv[2] ?? 'vendor');
  copyVendor(out);
  console.log(`vendor: copied ${Object.keys(vendorFiles).length} files to ${out}`);
}
