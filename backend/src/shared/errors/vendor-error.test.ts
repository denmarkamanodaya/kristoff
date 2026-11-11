import { describe, expect, it } from 'vitest';
import { errorTypeOf, upstreamError } from './vendor-error.js';

describe('vendor-error', () => {
  it('keeps the bounded error name, Unknown for non-errors', () => {
    expect([
      errorTypeOf(Object.assign(new Error('x'), { name: 'N'.repeat(80) })).length,
      errorTypeOf('s'),
    ]).toEqual([64, 'Unknown']);
  });

  it('builds UPSTREAM_UNAVAILABLE with operation and error.type', () => {
    const e = upstreamError(
      's3.GetObject',
      Object.assign(new Error('x'), { name: 'AccessDenied' }),
    );

    expect([e.code, e.message, e.details]).toEqual([
      'UPSTREAM_UNAVAILABLE',
      's3.GetObject failed',
      { operation: 's3.GetObject', 'error.type': 'AccessDenied' },
    ]);
  });
});
