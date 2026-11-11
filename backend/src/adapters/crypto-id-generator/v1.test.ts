import { describe, expect, it } from 'vitest';
import { cryptoIdGenerator } from './v1.js';

describe('crypto-id-generator v1', () => {
  it('makes 16-lowercase-hex ids that start with the epoch ms in 12 hex', () => {
    const ids = cryptoIdGenerator(() => Date.UTC(2026, 10, 12));

    const id = ids.newId();

    expect(id).toMatch(/^[a-f0-9]{16}$/);
    expect(id.slice(0, 12)).toBe((1794441600000).toString(16).padStart(12, '0'));
  });

  it('keeps ids strictly increasing within one millisecond and when the clock goes back', () => {
    let now = 1_800_000_000_000;
    const ids = cryptoIdGenerator(() => now);

    const made = Array.from({ length: 50 }, (_, i) => {
      if (i === 25) now -= 5;
      return ids.newId();
    });

    expect(new Set(made).size).toBe(50);
    expect(made.slice(0, 25)).toEqual([...made.slice(0, 25)].sort());
    expect(made.slice(25).every((id) => id.slice(0, 12) === made[24]?.slice(0, 12))).toBe(true);
  });

  it('makes ULIDs whose first 10 chars encode the time', () => {
    const ids = cryptoIdGenerator(() => Date.UTC(2026, 10, 12));

    const ulid = ids.newUlid();

    expect(ulid).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/);
    expect(ulid.slice(0, 10)).toBe('01M76GVQ00'); // 1794441600000 in Crockford base32, checked independently;
  });

  it('keeps ULIDs strictly increasing within one millisecond and when the clock goes back', () => {
    let now = 1_800_000_000_000;
    const ids = cryptoIdGenerator(() => now);

    const made = Array.from({ length: 50 }, (_, i) => {
      if (i === 25) now -= 5;
      return ids.newUlid();
    });

    expect([...made].sort()).toEqual(made);
    expect(new Set(made).size).toBe(50);
  });
});
