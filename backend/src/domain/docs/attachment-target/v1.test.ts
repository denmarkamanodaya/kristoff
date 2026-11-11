import { describe, expect, it } from 'vitest';
import { aDoc } from '../../../../test/support/builders.js';
import { attachmentTarget } from './v1.js';

const D = 'aaaaaaaaaaaaaaaa';
const PART = { n: 2, startPage: 101, endPage: 200, size: 9 };
const split = aDoc({ id: D, parts: [{ n: 1, startPage: 1, endPage: 100, size: 9 }, PART] });

describe('attachment-target v1', () => {
  it('attaches a ready doc without parts whole, and a part of a split doc', () => {
    expect(attachmentTarget({ docId: D, part: null }, aDoc({ id: D }))).toEqual({
      kind: 'whole',
      doc: aDoc({ id: D }),
    });
    expect(attachmentTarget({ docId: D, part: 2 }, split)).toEqual({
      kind: 'part',
      doc: split,
      part: PART,
    });
  });

  it.each([
    ['a split doc attached whole', { docId: D, part: null }, split],
    ['a part that does not exist', { docId: D, part: 3 }, split],
    ['a part of an unsplit doc', { docId: D, part: 1 }, aDoc({ id: D })],
    ['a processing doc', { docId: D, part: null }, aDoc({ id: D, status: 'processing' })],
    ['a failed doc', { docId: D, part: null }, aDoc({ id: D, status: 'failed' })],
    ['a missing doc', { docId: D, part: null }, undefined],
    ['another doc', { docId: D, part: null }, aDoc({ id: 'bbbbbbbbbbbbbbbb' })],
  ])('refuses %s', (_l, ref, doc) => {
    expect(attachmentTarget(ref, doc)).toBeUndefined();
  });
});
