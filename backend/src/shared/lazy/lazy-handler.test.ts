import { describe, expect, it } from 'vitest';
import { lazyHandler } from './lazy-handler.js';

describe('lazyHandler', () => {
  it('builds once and reuses the handler', async () => {
    let builds = 0;
    const entry = lazyHandler(() => {
      builds += 1;
      return Promise.resolve((n: number) => Promise.resolve(n * 2));
    });

    expect([await entry(1), await entry(2)]).toEqual([2, 4]);
    expect(builds).toBe(1);
  });

  it('rethrows a failed build and tries again next time', async () => {
    let builds = 0;
    const entry = lazyHandler<unknown, string>(() => {
      builds += 1;
      return builds === 1
        ? Promise.reject(new Error('invalid environment: TABLE_NAME'))
        : Promise.resolve(() => Promise.resolve('ok'));
    });

    await expect(entry(undefined)).rejects.toThrow('TABLE_NAME');
    expect(await entry(undefined)).toBe('ok');
  });

  it('passes the Lambda context through to the handler', async () => {
    const entry = lazyHandler(
      (): Promise<(n: number, c?: { getRemainingTimeInMillis(): number }) => Promise<number>> =>
        Promise.resolve((n, c) => Promise.resolve(n + (c?.getRemainingTimeInMillis() ?? 0))),
    );

    expect(await entry(1, { getRemainingTimeInMillis: () => 41 })).toBe(42);
  });
});
