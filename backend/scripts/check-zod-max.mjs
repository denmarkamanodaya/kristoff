#!/usr/bin/env node
// check-zod-max — every exported Zod schema in src/contracts/ must bound every string and array
// (`.max()` or `.length()`) and every number (`.min()` and `.max()`) (stack/security.md,
// stack/backend-lambda.md). Walks objects, records, arrays, unions, optionals, pipes and wrappers; an
// unknown schema type fails loudly instead of passing. Usage: node scripts/check-zod-max.mjs
import { readdirSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, importTs } from './lib/import-ts.mjs';

const CONTRACTS = join(ROOT, 'src', 'contracts');

const WRAPPERS = new Set([
  'optional',
  'nullable',
  'default',
  'readonly',
  'nonoptional',
  'catch',
  'prefault',
]);
const LEAVES = new Set(['literal', 'enum', 'boolean', 'null', 'undefined', 'never', 'transform']);

const isSchema = (v) => typeof v === 'object' && v !== null && '_zod' in v && v._zod?.def;
const checks = (def) => (def.checks ?? []).map((c) => c._zod?.def ?? c).map((c) => c.check);

const bounded = (has) => has('max_length', 'length_equals');

/** Per-type check: pushes problems and returns the children to walk as [schema, suffix]. */
const VISIT = {
  string: (def, has, flag) => (bounded(has) ? [] : flag('string without .max()')),
  number: (def, has, flag) => [
    ...(has('greater_than') ? [] : flag('number without .min()')),
    ...(has('less_than') ? [] : flag('number without .max()')),
  ],
  array: (def, has, flag) => [
    ...(bounded(has) ? [] : flag('array without .max()')),
    [def.element, '[]'],
  ],
  object: (def) => Object.entries(def.shape).map(([key, child]) => [child, `.${key}`]),
  // A record has no built-in size check: it needs a refine bounding its entry count (zod records
  // carry `.refine` as a custom check), and its keys and values are walked like any schema.
  record: (def, has, flag) => [
    ...(has('custom') ? [] : flag('record without a .refine() bounding its entries')),
    [def.keyType, '<key>'],
    [def.valueType, '<value>'],
  ],
  union: (def) => def.options.map((option, i) => [option, `|${i}`]),
  pipe: (def) => [
    [def.in, '<in>'],
    [def.out, '<out>'],
  ],
};

function walk(schema, path, problems, seen) {
  if (!isSchema(schema) || seen.has(schema)) return;
  seen.add(schema);
  const def = schema._zod.def;
  const has = (...names) => checks(def).some((c) => names.includes(c));
  const flag = (message) => {
    problems.push(`${path}: ${message}`);
    return [];
  };
  let children = [];
  if (VISIT[def.type]) children = VISIT[def.type](def, has, flag);
  else if (WRAPPERS.has(def.type)) children = [[def.innerType, '']];
  else if (!LEAVES.has(def.type)) flag(`unsupported type "${def.type}"; teach check-zod-max`);
  for (const [child, suffix] of children) walk(child, `${path}${suffix}`, problems, seen);
}

const sources = readdirSync(CONTRACTS).filter((f) => f.endsWith('.ts') && !f.endsWith('.test.ts'));
const mod = await importTs('zod-max', {
  contents: sources.map((f, i) => `export * as m${i} from './contracts/${f}';`).join('\n'),
});

const problems = [];
let count = 0;
sources.forEach((file, i) => {
  for (const [name, value] of Object.entries(mod[`m${i}`])) {
    if (!isSchema(value)) continue;
    count += 1;
    walk(value, `${file}:${name}`, problems, new Set());
  }
});
if (problems.length > 0) {
  console.error(`check:zod-max: ${problems.length} unbounded field(s)\n${problems.join('\n')}`);
  process.exit(1);
}
console.log(`check:zod-max: ${count} schemas, every string, array and number bounded`);
