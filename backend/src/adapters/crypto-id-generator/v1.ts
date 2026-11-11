// CryptoIdGenerator: the IdGenerator port on node:crypto. Ids and ULIDs are monotonic within one
// millisecond. The 16-hex id is 12 hex of epoch ms + 4 hex (random below 0x8000, then +1 within the same ms), so
// `DOC#<id>` sort keys order the library by creation (docs/contract.md Clarifications: newest
// first) and a user message and its answer always sort in write order.
// Contract: docs/contract.md (ids 16 lowercase hex; message ids ULID).
import { randomBytes } from 'node:crypto';
import type { IdGenerator } from '../../ports/id-generator.js';

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
const RANDOM_CHARS = 16;

const encodeTime = (ms: number): string => {
  let out = '';
  let rest = ms;
  for (let i = 0; i < 10; i += 1) {
    out = (CROCKFORD[rest % 32] ?? '0') + out;
    rest = Math.floor(rest / 32);
  }
  return out;
};

const randomDigits = (): number[] => [...randomBytes(RANDOM_CHARS)].map((b) => b % 32);

/** Adds one to a base-32 digit array (most significant first). */
const increment = (digits: readonly number[]): number[] => {
  const out = [...digits];
  for (let i = out.length - 1; i >= 0; i -= 1) {
    const next = (out[i] ?? 0) + 1;
    out[i] = next % 32;
    if (next < 32) break;
  }
  return out;
};

/**
 * Builds an id generator.
 *
 * @version 1
 * @since 2026-11-12
 * @change first version
 * @rollback n/a, first version
 * @param now - Epoch milliseconds (the system clock in production).
 * @returns The IdGenerator.
 */
export const cryptoIdGenerator = (now: () => number): IdGenerator => {
  let lastMs = -1;
  let lastRandom: number[] = [];
  let idMs = -1;
  let idTail = 0;
  return {
    newId: () => {
      const ms = Math.max(now(), idMs);
      idTail = ms === idMs ? (idTail + 1) % 0x10000 : randomBytes(2).readUInt16BE() & 0x7fff;
      idMs = ms;
      return ms.toString(16).padStart(12, '0').slice(-12) + idTail.toString(16).padStart(4, '0');
    },
    newUlid: () => {
      const ms = Math.max(now(), lastMs);
      lastRandom = ms === lastMs ? increment(lastRandom) : randomDigits();
      lastMs = ms;
      return encodeTime(ms) + lastRandom.map((d) => CROCKFORD[d] ?? '0').join('');
    },
  };
};
