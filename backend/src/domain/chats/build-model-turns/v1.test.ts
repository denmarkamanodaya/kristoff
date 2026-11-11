import { describe, expect, it } from 'vitest';
import { aMessage } from '../../../../test/support/builders.js';
import { buildModelTurns } from './v1.js';

const DOCS = [
  { id: 'aaaaaaaaaaaaaaaa', title: 'A', libraryTitle: 'A', url: 'https://s3/a', pageOffset: 0 },
  { id: 'bbbbbbbbbbbbbbbb', title: 'B', libraryTitle: 'B', url: 'https://s3/b', pageOffset: 0 },
];

describe('build-model-turns v1', () => {
  it('puts the docs in the first user turn with the cache breakpoint on the last doc only', () => {
    const turns = buildModelTurns([], 'Digest this.', DOCS, 20);

    expect(turns).toEqual([
      {
        role: 'user',
        blocks: [
          { kind: 'doc', title: 'A', url: 'https://s3/a', isCacheBreakpoint: false },
          { kind: 'doc', title: 'B', url: 'https://s3/b', isCacheBreakpoint: true },
          { kind: 'text', text: 'Digest this.' },
        ],
      },
    ]);
  });

  it('sends history as text turns before the question, docs in the first one', () => {
    const history = [
      aMessage({ role: 'user', text: 'Q1' }),
      aMessage({ role: 'assistant', text: 'A1 [1]' }),
    ];

    const turns = buildModelTurns(history, 'Q2', DOCS.slice(0, 1), 20);

    expect(turns.map((t) => [t.role, t.blocks.length])).toEqual([
      ['user', 2],
      ['assistant', 1],
      ['user', 1],
    ]);
    expect(turns[1]?.blocks[0]).toEqual({ kind: 'text', text: 'A1 [1]' });
  });

  it('keeps only the last maxHistoryTurns messages and starts the window on a user turn', () => {
    const history = [
      aMessage({ role: 'user', text: 'Q1' }),
      aMessage({ role: 'assistant', text: 'A1' }),
      aMessage({ role: 'user', text: 'Q2' }),
      aMessage({ role: 'assistant', text: 'A2' }),
    ];

    const turns = buildModelTurns(history, 'Q3', [], 3);

    expect(turns.map((t) => t.blocks.map((b) => (b.kind === 'text' ? b.text : 'doc')))).toEqual([
      ['Q2'],
      ['A2'],
      ['Q3'],
    ]);
  });

  it('sends no history when maxHistoryTurns is 0', () => {
    const turns = buildModelTurns([aMessage({ text: 'Q1' })], 'Q2', [], 0);

    expect(turns).toEqual([{ role: 'user', blocks: [{ kind: 'text', text: 'Q2' }] }]);
  });

  it('joins consecutive same-role turns and replaces empty text', () => {
    const history = [
      aMessage({ role: 'user', text: 'Q1' }),
      aMessage({ role: 'assistant', text: '' }),
      aMessage({ role: 'user', text: 'Q2' }),
    ];

    const turns = buildModelTurns(history, 'Q3', [], 20);

    expect(turns.map((t) => t.blocks[0])).toEqual([
      { kind: 'text', text: 'Q1' },
      { kind: 'text', text: '(no answer)' },
      { kind: 'text', text: 'Q2\n\nQ3' },
    ]);
  });
});
