import { describe, expect, it } from 'vitest';
import { mapCitation } from './v1.js';

const DOCS = [
  { id: 'aaaaaaaaaaaaaaaa', title: 'People v. Cruz', url: 'https://s3/a' },
  { id: 'bbbbbbbbbbbbbbbb', title: 'Civil Code', url: 'https://s3/b' },
];

describe('map-citation v1', () => {
  it('ties the citation to the indexed doc and makes the end page inclusive', () => {
    const source = {
      documentIndex: 1,
      documentTitle: 'Civil Code',
      startPage: 3,
      endPageExclusive: 5,
      citedText: 'Art. 19.',
    };

    expect(mapCitation(source, DOCS, 2)).toEqual({
      n: 2,
      docId: 'bbbbbbbbbbbbbbbb',
      docTitle: 'Civil Code',
      page: 3,
      endPage: 4,
      quote: 'Art. 19.',
    });
  });

  it('falls back to the library title, then to "Document", with null pages', () => {
    const source = {
      documentIndex: 0,
      documentTitle: null,
      startPage: null,
      endPageExclusive: null,
      citedText: 'q',
    };

    expect(mapCitation(source, DOCS, 1)).toMatchObject({
      docTitle: 'People v. Cruz',
      page: null,
      endPage: null,
    });
    expect(mapCitation({ ...source, documentIndex: 9 }, DOCS, 1)).toMatchObject({
      docId: null,
      docTitle: 'Document',
    });
  });

  it('clamps the title to 200 and the quote to 4000 characters', () => {
    const source = {
      documentIndex: 0,
      documentTitle: 't'.repeat(300),
      startPage: 1,
      endPageExclusive: 2,
      citedText: 'q'.repeat(5000),
    };

    const citation = mapCitation(source, DOCS, 1);

    expect([citation.docTitle.length, citation.quote.length]).toEqual([200, 4000]);
  });
});
