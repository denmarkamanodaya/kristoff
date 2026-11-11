import { describe, expect, it } from 'vitest';
import { AppError, appError } from '../errors/app-error.js';
import { err, ok } from './result.js';

describe('result and AppError', () => {
  it('builds the two variants', () => {
    expect(ok(1)).toEqual({ ok: true, value: 1 });
    expect(err('x')).toEqual({ ok: false, error: 'x' });
  });

  it('carries a stable code and optional details, never an HTTP status', () => {
    const e = appError('DOC_NOT_FOUND', 'no clip', { count: 1 });

    expect(e).toBeInstanceOf(AppError);
    expect([e.code, e.message, e.details]).toEqual(['DOC_NOT_FOUND', 'no clip', { count: 1 }]);
    expect(e).not.toHaveProperty('status');
  });
});
