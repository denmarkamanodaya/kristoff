import { describe, expect, it } from 'vitest';
import { appError } from '../../shared/errors/app-error.js';
import { err, ok } from '../../shared/result/result.js';
import { httpDeps } from '../../../test/support/http-events.js';
import { createDocsSplitHandler } from './v1.js';

describe('docs-split handler v1', () => {
  it('runs split-doc with the event ids and logs one info line', async () => {
    const deps = httpDeps();
    const seen: unknown[] = [];
    const handler = createDocsSplitHandler({
      log: deps.log,
      now: deps.now,
      splitDoc: (u, d) => (seen.push([u, d]), Promise.resolve(ok('ready' as const))),
    });

    await handler({ uid: 'kristoff', docId: 'aaaaaaaaaaaaaaaa' });

    expect(seen).toEqual([['kristoff', 'aaaaaaaaaaaaaaaa']]);
    expect(deps.lines).toEqual([
      {
        level: 'info',
        event: 'split.request',
        operation: 'docs-split',
        outcome: 'ready',
        durationMs: 0,
      },
    ]);
  });

  it('resolves (no Lambda retry) and logs a warn when the split failed', async () => {
    const deps = httpDeps();
    const handler = createDocsSplitHandler({
      log: deps.log,
      now: deps.now,
      splitDoc: () => Promise.resolve(err(appError('UPLOAD_INVALID', 'x'))),
    });

    await expect(handler({ uid: 'kristoff', docId: 'aaaaaaaaaaaaaaaa' })).resolves.toBeUndefined();
    expect(deps.lines[0]).toMatchObject({ level: 'warn', outcome: 'UPLOAD_INVALID' });
  });

  it.each([
    ['a malformed docId', { uid: 'kristoff', docId: 'x' }],
    ['an extra key', { uid: 'k', docId: 'aaaaaaaaaaaaaaaa', x: 1 }],
    ['no object', 'x'],
  ])('logs an error and does not split for %s', async (_l, event) => {
    const deps = httpDeps();
    const handler = createDocsSplitHandler({
      log: deps.log,
      now: deps.now,
      splitDoc: () => Promise.reject(new Error('called')),
    });

    await handler(event);

    expect(deps.lines).toEqual([
      {
        level: 'error',
        event: 'split.request',
        operation: 'docs-split',
        outcome: 'VALIDATION_FAILED',
      },
    ]);
  });
});
