// Validates contracts/fixtures/eval/questions.json against contracts/eval-questions.schema.json
// with a tiny JSON Schema subset validator, checks the eval-set rules (counts, languages, docs)
// and, when desktop/data/phlaws.db exists, that every target provision is in the index.
// Usage: node --test desktop/scripts/eval-questions.test.mjs
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { DOCS_DIR, PAGE_BREAK, buildAll } from './eval-docs-to-pdf.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const CONTRACTS = join(here, '../../contracts');
const schema = JSON.parse(readFileSync(join(CONTRACTS, 'eval-questions.schema.json'), 'utf8'));
const loadSet = () => JSON.parse(readFileSync(join(CONTRACTS, 'fixtures/eval/questions.json'), 'utf8'));
const DB = join(here, '../data/phlaws.db');

/** Errors for `value` against `s`. Supports the keywords the eval schema uses, nothing else. */
function validate(s, value, path = '$') {
  if (s.$ref) return validate(schema.$defs[s.$ref.replace('#/$defs/', '')], value, path);
  if (s.oneOf) {
    const passing = s.oneOf.filter((alt) => validate(alt, value, path).length === 0).length;
    return passing === 1 ? [] : [`${path}: matches ${passing} of oneOf`];
  }
  const errs = [];
  const isObj = value !== null && typeof value === 'object' && !Array.isArray(value);
  const types = { object: isObj, array: Array.isArray(value), string: typeof value === 'string',
    integer: Number.isInteger(value) };
  if (s.type && !types[s.type]) return [`${path}: not ${s.type}`];
  if ('const' in s && value !== s.const) errs.push(`${path}: not ${s.const}`);
  if (s.enum && !s.enum.includes(value)) errs.push(`${path}: not in enum`);
  if (typeof value === 'string') {
    if (s.minLength !== undefined && value.length < s.minLength) errs.push(`${path}: too short`);
    if (s.maxLength !== undefined && value.length > s.maxLength) errs.push(`${path}: too long`);
    if (s.pattern && !new RegExp(s.pattern).test(value)) errs.push(`${path}: bad pattern`);
  }
  if (Number.isInteger(value)) {
    if (s.minimum !== undefined && value < s.minimum) errs.push(`${path}: below minimum`);
    if (s.maximum !== undefined && value > s.maximum) errs.push(`${path}: above maximum`);
  }
  if (Array.isArray(value)) {
    if (s.minItems !== undefined && value.length < s.minItems) errs.push(`${path}: too few items`);
    if (s.maxItems !== undefined && value.length > s.maxItems) errs.push(`${path}: too many items`);
    if (s.items) value.forEach((v, i) => errs.push(...validate(s.items, v, `${path}[${i}]`)));
  }
  if (isObj) {
    for (const k of s.required ?? []) if (!(k in value)) errs.push(`${path}: missing ${k}`);
    for (const [k, v] of Object.entries(value)) {
      if (s.properties?.[k]) errs.push(...validate(s.properties[k], v, `${path}.${k}`));
      else if (s.additionalProperties === false) errs.push(`${path}: unknown ${k}`);
    }
  }
  return errs;
}

test('the eval set matches its schema', () => {
  assert.deepEqual(validate(schema, loadSet()), []);
});

test('the validator rejects broken eval sets', () => {
  const twoTargets = loadSet();
  twoTargets.questions[0].targets.push(twoTargets.questions[0].targets[0]);
  const noDoc = loadSet();
  delete noDoc.questions[39].doc;
  const badKind = loadSet();
  badKind.questions[20].kind = 'essay';

  for (const broken of [twoTargets, noDoc, badKind]) {
    assert.notDeepEqual(validate(schema, broken), []);
  }
});

test('the set has 20 look-ups, 10 concepts, 10 mixed and at least 8 non-English questions', () => {
  const { questions } = loadSet();
  const count = (kind) => questions.filter((q) => q.kind === kind).length;
  const nonEnglish = questions.filter((q) => q.language !== 'en');

  assert.deepEqual([count('lookup'), count('concept'), count('mixed')], [20, 10, 10]);
  assert.equal(new Set(questions.map((q) => q.id)).size, 40);
  assert.ok(nonEnglish.length >= 8, `${nonEnglish.length} non-English`);
  assert.equal(new Set(nonEnglish.map((q) => q.kind)).size, 3, 'Taglish/Tagalog in every kind');
});

test('every mixed doc is marked fictional and holds its expected points on the expected page', () => {
  for (const q of loadSet().questions.filter((x) => x.kind === 'mixed')) {
    const text = readFileSync(join(DOCS_DIR, q.doc.file.replace('docs/', '')), 'utf8');
    const pages = text.split(PAGE_BREAK);

    assert.ok(text.startsWith('FICTIONAL DECISION - FOR EVALUATION ONLY.'), q.id);
    assert.equal(pages.length, q.doc.pages, q.id);
    const page = pages[q.docExpect.page - 1].replace(/\s+/g, ' ');
    for (const point of q.docExpect.mustMention) {
      assert.ok(page.includes(point), `${q.id}: "${point}"`);
    }
  }
});

test('the generator writes one PDF per doc with the expected page count', () => {
  const out = mkdtempSync(join(tmpdir(), 'kristoff-eval-pdfs-'));

  const written = buildAll(out);

  assert.equal(written.length, 10);
  for (const path of written) {
    const pdf = readFileSync(path, 'latin1');
    assert.ok(pdf.startsWith('%PDF-1.4') && pdf.trimEnd().endsWith('%%EOF'), path);
    assert.equal((pdf.match(/\/Type \/Page /g) ?? []).length, 2, path);
  }
});

test('every target provision exists in phlaws.db', { skip: !existsSync(DB) && 'desktop/data/phlaws.db not built' }, () => {
  const sql = (s) => `'${s.replaceAll("'", "''")}'`;
  for (const q of loadSet().questions) {
    for (const t of q.targets) {
      const query = `SELECT count(*) FROM chunks c JOIN laws l ON l.id = c.law_id WHERE c.part = 0 ` +
        `AND l.identifier = ${sql(t.identifier)} AND c.label = ${sql(t.label)}`;
      const n = execFileSync('sqlite3', ['-readonly', DB, query], { encoding: 'utf8' }).trim();
      assert.notEqual(n, '0', `${q.id}: ${t.identifier} · ${t.label}`);
    }
  }
});
