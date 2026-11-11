import { describe, expect, it } from 'vitest';
import { projectName } from './v1.js';

describe('project-name v1', () => {
  it('trims the name', () => {
    expect(projectName('  Torts ')).toEqual({ ok: true, value: 'Torts' });
  });

  it.each([
    ['blank', '   '],
    ['81 characters', 'x'.repeat(81)],
  ])('refuses a %s name', (_l, raw) => {
    expect(projectName(raw)).toMatchObject({ ok: false, error: { code: 'VALIDATION_FAILED' } });
  });
});
