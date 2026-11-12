#!/usr/bin/env node
// Writes desktop/THIRD_PARTY_NOTICES: every crate the shipped app links (from `cargo metadata`,
// normal dependencies of kristoff-app on aarch64-apple-darwin, as pinned by Cargo.lock), the two
// web libraries in the desktop bundle, the PH laws dataset, the OmniCorpus dataset (ODC-By, Phase 7)
// and the e5 model, then each distinct
// licence text found in those packages. No new tool: cargo, node and the local registry only.
// Phase 5 §2 (docs/desktop/phases/phase-5-eval-package.md).
// Usage: node desktop/scripts/third-party-notices.mjs [--check]   (--check fails if stale)
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const desktop = join(dirname(fileURLToPath(import.meta.url)), '..');
const web = join(desktop, '..', 'web');
const out = join(desktop, 'THIRD_PARTY_NOTICES');
const LICENCE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;

function metadata() {
  const json = execFileSync(
    'cargo',
    ['metadata', '--format-version', '1', '--filter-platform', 'aarch64-apple-darwin', '--locked'],
    { cwd: desktop, maxBuffer: 256 * 1024 * 1024 },
  );
  return JSON.parse(json);
}

/** Every external package reachable from kristoff-app through normal (shipped) dependencies. */
function shippedCrates(meta) {
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const root = meta.packages.find((p) => p.name === 'kristoff-app').id;
  const seen = new Set();
  const stack = [root];
  while (stack.length > 0) {
    const id = stack.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id).deps) {
      if (dep.dep_kinds.some((k) => k.kind === null)) stack.push(dep.pkg);
    }
  }
  return [...seen]
    .map((id) => byId.get(id))
    .filter((p) => p.source !== null)
    .map((p) => ({
      name: p.name,
      version: p.version,
      licence: p.license ?? `see ${p.license_file ?? 'package'}`,
      home: p.repository ?? p.homepage ?? '',
      dir: dirname(p.manifest_path),
    }))
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

function webLibraries() {
  return ['dompurify', 'marked'].map((name) => {
    const dir = join(web, 'node_modules', name);
    const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
    const home = typeof pkg.repository === 'object' ? pkg.repository.url : (pkg.repository ?? '');
    return { name, version: pkg.version, licence: pkg.license, home, dir };
  });
}

function licenceTexts(pkg) {
  if (!existsSync(pkg.dir)) return [];
  return readdirSync(pkg.dir)
    .filter((f) => LICENCE_FILE.test(f) && statSync(join(pkg.dir, f)).isFile())
    .sort()
    .map((f) => ({ file: f, text: readFileSync(join(pkg.dir, f), 'utf8').trim() }));
}

const DATA_AND_MODEL = `Data and model (bundled as resources)

PH laws dataset: endomorphosis/ipfs_philippines_laws, revision
66ffcff9f6fa1288ca5247d7b20141dc841beb45 (snapshot 2026-09-16), https://huggingface.co/datasets/endomorphosis/ipfs_philippines_laws
  Licence on the dataset card: "other" (ph-official-gazette-texts): "Official Philippine legislative
  texts; Official Gazette authentic text prevails. Not Lawphil. Not Chan Robles. Not legal advice."
  The texts are Republic Acts and the 1987 Constitution from the Official Gazette and the Senate LRB.
  Under the Philippine Intellectual Property Code (RA 8293, Sec. 176) no copyright subsists in works
  of the Government; prior approval is required only to exploit them for profit. This app is for
  private, non-commercial use.

Philippine OmniCorpus: mongramosjr/philippine-omnicorpus, revision
bb0626ce527b159a32a6baefabee0ea1e0e01d2f, https://huggingface.co/datasets/mongramosjr/philippine-omnicorpus
  Ramos Jr., M. (2024). Philippine OmniCorpus [Data set]. Hugging Face.
  Licence: Open Data Commons Attribution License (ODC-By) v1.0,
  https://opendatacommons.org/licenses/by/1-0/
  Used for the codes and older laws as originally enacted (in phlaws.db) and for the Supreme Court
  decisions (phcases.db). The texts are works of the Philippine Government; see the note above.

multilingual-e5-small: intfloat/multilingual-e5-small, revision
614241f622f53c4eeff9890bdc4f31cfecc418b3, https://huggingface.co/intfloat/multilingual-e5-small
  Licence: MIT. Weights shipped converted to f16.`;

function render() {
  const crates = shippedCrates(metadata());
  const libs = webLibraries();
  const all = [...crates, ...libs];
  const list = (pkgs) => pkgs.map((p) => `${p.name} ${p.version} | ${p.licence} | ${p.home}`).join('\n');
  const texts = new Map();
  for (const p of all) {
    for (const { file, text } of licenceTexts(p)) {
      const users = texts.get(text) ?? [];
      users.push(`${p.name} ${p.version} (${file})`);
      texts.set(text, users);
    }
  }
  const missing = all.filter((p) => licenceTexts(p).length === 0).map((p) => `${p.name} ${p.version}`);
  const blocks = [...texts.entries()]
    .sort((a, b) => a[1][0].localeCompare(b[1][0]))
    .map(([text, users]) => `---- ${users.join(', ')}\n\n${text}`);
  return [
    'Kristoff Desktop: third-party notices',
    'Generated by desktop/scripts/third-party-notices.mjs from Cargo.lock and web/package-lock.json.',
    '',
    DATA_AND_MODEL,
    '',
    `Web libraries in the app window (${libs.length})`,
    '',
    list(libs),
    '',
    `Rust crates linked into the app (${crates.length}): name version | licence | source`,
    '',
    list(crates),
    '',
    'Packages that ship no licence file of their own (their licence is the SPDX expression above):',
    missing.length > 0 ? missing.join(', ') : '(none)',
    '',
    `Licence texts (${blocks.length} distinct), each followed by the packages it comes from`,
    '',
    blocks.join('\n\n'),
    '',
  ].join('\n');
}

const text = render();
if (process.argv.includes('--check')) {
  const current = existsSync(out) ? readFileSync(out, 'utf8') : '';
  if (current !== text) {
    console.error('THIRD_PARTY_NOTICES is stale: run node desktop/scripts/third-party-notices.mjs');
    process.exit(1);
  }
  console.log('THIRD_PARTY_NOTICES is up to date');
} else {
  writeFileSync(out, text);
  console.log(`wrote ${out} (${text.length} bytes)`);
}
