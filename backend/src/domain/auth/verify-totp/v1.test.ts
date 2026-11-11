import { describe, expect, it } from 'vitest';
import { decodeBase32, hotp, matchTotpStep, stepAt } from './v1.js';

// RFC 6238 Appendix B, SHA-1 secret "12345678901234567890" (ASCII).
const RFC_SECRET = Buffer.from('12345678901234567890', 'ascii');
const RFC_VECTORS: readonly (readonly [number, string])[] = [
  [59, '94287082'],
  [1111111109, '07081804'],
  [1111111111, '14050471'],
  [1234567890, '89005924'],
  [2000000000, '69279037'],
  [20000000000, '65353130'],
];

describe('verify-totp v1', () => {
  it.each(RFC_VECTORS)('matches the RFC 6238 SHA-1 vector at T=%i', (seconds, expected) => {
    expect(hotp(RFC_SECRET, stepAt(seconds * 1000), 8)).toBe(expected);
  });

  it('decodes base32 of the RFC secret, ignoring case, padding and spaces', () => {
    const encoded = 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ';

    expect(decodeBase32(encoded)).toEqual(RFC_SECRET);
    expect(decodeBase32('gezd gnbv gy3t qojq gezd gnbv gy3t qojq==')).toEqual(RFC_SECRET);
  });

  it('rejects non-base32 text and empty text', () => {
    expect([decodeBase32('GEZ1'), decodeBase32('  ')]).toEqual([undefined, undefined]);
  });

  it('accepts the current step and one step either side, returning the matched step', () => {
    const now = 1111111111 * 1000;
    const current = stepAt(now);

    expect(matchTotpStep(RFC_SECRET, hotp(RFC_SECRET, current), now)).toBe(current);
    expect(matchTotpStep(RFC_SECRET, hotp(RFC_SECRET, current - 1), now)).toBe(current - 1);
    expect(matchTotpStep(RFC_SECRET, hotp(RFC_SECRET, current + 1), now)).toBe(current + 1);
  });

  it('rejects a code two steps away and a code that is not six digits', () => {
    const now = 1111111111 * 1000;
    const current = stepAt(now);

    expect(matchTotpStep(RFC_SECRET, hotp(RFC_SECRET, current - 2), now)).toBeUndefined();
    expect(matchTotpStep(RFC_SECRET, '12345', now)).toBeUndefined();
    expect(matchTotpStep(RFC_SECRET, 'abcdef', now)).toBeUndefined();
  });
});
