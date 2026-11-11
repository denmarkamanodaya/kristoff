import { createHmac } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { signSession, verifySessionToken } from './v1.js';

const KEY = Buffer.alloc(32, 7);
const PAYLOAD = { sub: 'kristoff', iat: 1000, exp: 2000 };

describe('session-token v1', () => {
  it('signs base64url(payload).base64url(HMAC-SHA256) exactly as the contract states', () => {
    const payloadPart = Buffer.from(JSON.stringify(PAYLOAD)).toString('base64url');
    const macPart = createHmac('sha256', KEY).update(payloadPart).digest('base64url');

    expect(signSession(KEY, PAYLOAD)).toBe(`${payloadPart}.${macPart}`);
  });

  it('verifies its own token before expiry', () => {
    expect(verifySessionToken(KEY, signSession(KEY, PAYLOAD), 1999)).toEqual(PAYLOAD);
  });

  it('rejects an expired token at and after exp', () => {
    expect(verifySessionToken(KEY, signSession(KEY, PAYLOAD), 2000)).toBeUndefined();
  });

  it('rejects a token signed with another key (forged)', () => {
    const forged = signSession(Buffer.alloc(32, 8), PAYLOAD);

    expect(verifySessionToken(KEY, forged, 1500)).toBeUndefined();
  });

  it('rejects a payload swapped under a valid MAC', () => {
    const [, macPart] = signSession(KEY, PAYLOAD).split('.');
    const other = Buffer.from(JSON.stringify({ ...PAYLOAD, sub: 'eve' })).toString('base64url');

    expect(verifySessionToken(KEY, `${other}.${String(macPart)}`, 1500)).toBeUndefined();
  });

  it.each([
    ['no dot', 'abc'],
    ['three parts', 'a.b.c'],
    ['too long', `${'a'.repeat(1100)}.b`],
    ['mac of wrong length', 'eyJ9.AA'],
  ])('rejects a malformed token (%s)', (_label, token) => {
    expect(verifySessionToken(KEY, token, 1500)).toBeUndefined();
  });

  it('rejects a correctly signed payload that is not the session shape', () => {
    const payloadPart = Buffer.from('{"sub":1}').toString('base64url');
    const macPart = createHmac('sha256', KEY).update(payloadPart).digest('base64url');
    const notJson = Buffer.from('not json').toString('base64url');
    const notJsonMac = createHmac('sha256', KEY).update(notJson).digest('base64url');

    expect(verifySessionToken(KEY, `${payloadPart}.${macPart}`, 1500)).toBeUndefined();
    expect(verifySessionToken(KEY, `${notJson}.${notJsonMac}`, 1500)).toBeUndefined();
  });
});
