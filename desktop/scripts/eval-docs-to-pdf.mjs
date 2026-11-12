#!/usr/bin/env node
// Turns the fictional eval decisions (contracts/fixtures/eval/docs/*.txt) into PDFs for the
// Phase 5 mixed questions (docs/desktop/phases/phase-5-eval-package.md §1). No dependencies:
// it writes a minimal PDF 1.4 by hand (Courier 10 pt, US Letter), one page per
// "<<<PAGE BREAK>>>" section, with no timestamps, so the same text always gives the same bytes.
// Usage: node desktop/scripts/eval-docs-to-pdf.mjs [out-dir]   (default desktop/target/eval/pdfs)
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
export const DOCS_DIR = join(here, '../../contracts/fixtures/eval/docs');
export const PAGE_BREAK = '<<<PAGE BREAK>>>';
const LINE_HEIGHT = 12;
const TOP = 760;
const LEFT = 54;
const MAX_LINES = 58;

/** Splits a source text into pages of lines. Throws on non-ASCII (Courier's WinAnsi subset). */
export function pagesOf(text) {
  if (/[^\x09\x0a\x0d\x20-\x7e]/.test(text)) throw new Error('eval docs must be plain ASCII');
  return text.split(PAGE_BREAK).map((page) => {
    const lines = page.replace(/^\n+|\n+$/g, '').split('\n');
    if (lines.length > MAX_LINES) throw new Error(`page has ${lines.length} lines (max ${MAX_LINES})`);
    return lines;
  });
}

const escape = (s) => s.replace(/[\\()]/g, (c) => `\\${c}`);

/** Builds the PDF bytes for pages of lines. */
export function pdfOf(pages) {
  const objects = [];
  const add = (body) => objects.push(body) && objects.length;
  const catalog = add('');
  const pagesRef = add('');
  const font = add('<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>');
  const kids = pages.map((lines) => {
    const ops = lines.map((l, i) => `1 0 0 1 ${LEFT} ${TOP - i * LINE_HEIGHT} Tm (${escape(l)}) Tj`);
    const stream = `BT /F1 10 Tf\n${ops.join('\n')}\nET`;
    const content = add(`<< /Length ${Buffer.byteLength(stream)} >>\nstream\n${stream}\nendstream`);
    return add(`<< /Type /Page /Parent ${pagesRef} 0 R /MediaBox [0 0 612 792] ` +
      `/Resources << /Font << /F1 ${font} 0 R >> >> /Contents ${content} 0 R >>`);
  });
  objects[catalog - 1] = `<< /Type /Catalog /Pages ${pagesRef} 0 R >>`;
  objects[pagesRef - 1] = `<< /Type /Pages /Kids [${kids.map((k) => `${k} 0 R`).join(' ')}] /Count ${kids.length} >>`;
  let out = '%PDF-1.4\n';
  const offsets = objects.map((body, i) => {
    const at = Buffer.byteLength(out);
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
    return at;
  });
  const xref = Buffer.byteLength(out);
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  out += offsets.map((o) => `${String(o).padStart(10, '0')} 00000 n \n`).join('');
  out += `trailer\n<< /Size ${objects.length + 1} /Root ${catalog} 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(out, 'latin1');
}

/** Writes one PDF per doc into `outDir`; returns the written paths. */
export function buildAll(outDir) {
  mkdirSync(outDir, { recursive: true });
  return readdirSync(DOCS_DIR).filter((f) => f.endsWith('.txt')).sort().map((f) => {
    const target = join(outDir, `${basename(f, '.txt')}.pdf`);
    writeFileSync(target, pdfOf(pagesOf(readFileSync(join(DOCS_DIR, f), 'utf8'))));
    return target;
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const outDir = process.argv[2] ?? join(here, '../target/eval/pdfs');
  for (const path of buildAll(outDir)) process.stdout.write(`${path}\n`);
}
