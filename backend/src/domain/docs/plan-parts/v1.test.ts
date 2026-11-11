import { describe, expect, it } from 'vitest';
import { halveRange, initialRanges, needsParts, partFits } from './v1.js';

const MIB = 1024 * 1024;

describe('plan-parts v1', () => {
  it('splits only above 100 pages or 30 MiB', () => {
    expect([needsParts(100, 30 * MIB), needsParts(101, 1), needsParts(1, 30 * MIB + 1)]).toEqual([
      false,
      true,
      true,
    ]);
  });

  it('cuts consecutive ranges of at most 100 pages covering every page', () => {
    expect(initialRanges(250)).toEqual([
      { startPage: 1, endPage: 100 },
      { startPage: 101, endPage: 200 },
      { startPage: 201, endPage: 250 },
    ]);
    expect(initialRanges(100)).toEqual([{ startPage: 1, endPage: 100 }]);
  });

  it('halves a range, and cannot halve one page', () => {
    expect(halveRange({ startPage: 101, endPage: 200 })).toEqual([
      { startPage: 101, endPage: 150 },
      { startPage: 151, endPage: 200 },
    ]);
    expect(halveRange({ startPage: 7, endPage: 8 })).toEqual([
      { startPage: 7, endPage: 7 },
      { startPage: 8, endPage: 8 },
    ]);
    expect(halveRange({ startPage: 7, endPage: 7 })).toBeUndefined();
  });

  it('accepts a part of exactly 30 MiB', () => {
    expect([partFits(30 * MIB), partFits(30 * MIB + 1)]).toEqual([true, false]);
  });

  it('covers every page exactly once for any page count (property, seeded)', () => {
    let seed = 42;
    const next = () => ((seed = (seed * 1103515245 + 12345) % 2 ** 31), seed);
    for (let i = 0; i < 200; i += 1) {
      const pages = (next() % 2000) + 1;
      const covered = initialRanges(pages).flatMap((r) =>
        Array.from({ length: r.endPage - r.startPage + 1 }, (_, k) => r.startPage + k),
      );
      expect(covered, `seed run ${String(i)} pages ${String(pages)}`).toEqual(
        Array.from({ length: pages }, (_, k) => k + 1),
      );
    }
  });
});
