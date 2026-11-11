import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { describe, expect, it } from 'vitest';
import { zipOne } from './zip.mjs';

describe('zipOne', () => {
  it('gives byte-identical zips for the same input', () => {
    const data = Buffer.from('export const handler = () => 1;\n'.repeat(50));

    expect(zipOne('index.mjs', data).equals(zipOne('index.mjs', data))).toBe(true);
  });

  it('stamps 2000-01-01 00:00 in the local and the central header', () => {
    const zip = zipOne('index.mjs', Buffer.from('console.log("hi");\n'));
    const end = zip.length - 22;
    const central = zip.readUInt32LE(end + 16);
    // DOS time and date fields: local header at 10/12, central header at 12/14 past its start.
    const stamps = [
      { sig: zip.readUInt32LE(0), time: zip.readUInt16LE(10), date: zip.readUInt16LE(12) },
      {
        sig: zip.readUInt32LE(central),
        time: zip.readUInt16LE(central + 12),
        date: zip.readUInt16LE(central + 14),
      },
    ];
    const ymd = (date) => [1980 + (date >> 9), (date >> 5) & 0x0f, date & 0x1f];

    expect(zip.readUInt32LE(end)).toBe(0x06054b50);
    expect(stamps.map((s) => s.sig)).toEqual([0x04034b50, 0x02014b50]);
    for (const { time, date } of stamps) {
      expect(time).toBe(0);
      expect(ymd(date)).toEqual([2000, 1, 1]);
    }
  });

  it('writes a zip that unzip extracts back to the exact bytes', () => {
    const data = Buffer.from('console.log("hi");\n');
    const dir = mkdtempSync(join(tmpdir(), 'zip-test-'));
    try {
      writeFileSync(join(dir, 'a.zip'), zipOne('index.mjs', data));

      execFileSync('unzip', ['-q', join(dir, 'a.zip'), '-d', join(dir, 'out')]);

      expect(readFileSync(join(dir, 'out', 'index.mjs')).equals(data)).toBe(true);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
