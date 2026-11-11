import { describe, expect, it } from 'vitest';
import { isAttachmentOf, parseAttachmentId } from './v1.js';

const D = 'aaaaaaaaaaaaaaaa';

describe('attachment-id v1', () => {
  it('parses a whole-doc id and a part id', () => {
    expect([parseAttachmentId(D), parseAttachmentId(`${D}:p12`)]).toEqual([
      { docId: D, part: null },
      { docId: D, part: 12 },
    ]);
  });

  it.each([
    ['part 0', `${D}:p0`],
    ['4-digit part', `${D}:p1000`],
    ['short id', 'abc'],
    ['upper case', D.toUpperCase()],
  ])('rejects %s', (_l, id) => {
    expect(parseAttachmentId(id)).toBeUndefined();
  });

  it('matches the doc and its parts only', () => {
    expect([
      isAttachmentOf(D, D),
      isAttachmentOf(`${D}:p2`, D),
      isAttachmentOf('bbbbbbbbbbbbbbbb', D),
    ]).toEqual([true, true, false]);
  });
});
