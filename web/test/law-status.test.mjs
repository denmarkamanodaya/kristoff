// Tests for ui/law-status.js: every hinted status label is one the status-line regex accepts, and a
// law quote's label is split off as the badge (docs/contract.md, "Law status line").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { STATUS_HINTS, STATUS_LINE, lawStatus } from '../ui/law-status.js';

const LABELS = ['As enacted', 'As promulgated', 'As issued', 'As signed', 'Constitutional records', 'Court reference'];

test('every hinted label matches the status-line regex exactly', () => {
  for (const label of Object.keys(STATUS_HINTS)) {
    const m = STATUS_LINE.exec(`[${label}] text`);
    assert.ok(m, label);
    assert.equal(m[1], label);
  }
});

test('the OmniCorpus labels each have hover text', () => {
  assert.deepEqual(Object.keys(STATUS_HINTS).sort(), [...LABELS].sort());
  for (const label of LABELS) assert.ok(STATUS_HINTS[label].length > 0, label);
});

test('a law quote loses its label, which becomes the status', () => {
  for (const label of LABELS) {
    const c = { kind: 'law', quote: `[${label}] Article 1. Text.` };
    assert.deepEqual(lawStatus(c), { status: label, quote: 'Article 1. Text.' });
  }
});

test('doc and case quotes are never parsed, and a law quote without a label has no status', () => {
  assert.deepEqual(lawStatus({ kind: 'doc', quote: '[As enacted] x' }), { status: null, quote: '[As enacted] x' });
  assert.deepEqual(lawStatus({ kind: 'case', quote: '[As enacted] x' }), { status: null, quote: '[As enacted] x' });
  assert.deepEqual(lawStatus({ quote: '[Repealed] x' }), { status: null, quote: '[Repealed] x' });
  assert.deepEqual(lawStatus({ kind: 'law', quote: 'Sec. 1. Text.' }), { status: null, quote: 'Sec. 1. Text.' });
});
