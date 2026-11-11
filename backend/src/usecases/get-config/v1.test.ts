import { describe, expect, it } from 'vitest';
import { getConfig } from './v1.js';

describe('get-config v1', () => {
  it('returns the first allowed model as default and every model with its label, in order', () => {
    const run = getConfig({
      modelIds: ['claude-sonnet-5-5', 'claude-opus-5-5', 'claude-new-6'],
      labels: { 'claude-sonnet-5-5': 'Sonnet 5.5', 'claude-opus-5-5': 'Opus 5.5' },
      maxAttachedDocs: 5,
      dailyQuestionCap: 100,
    });

    expect(run()).toEqual({
      model: 'claude-sonnet-5-5',
      models: [
        { id: 'claude-sonnet-5-5', label: 'Sonnet 5.5' },
        { id: 'claude-opus-5-5', label: 'Opus 5.5' },
        { id: 'claude-new-6', label: 'claude-new-6' },
      ],
      maxAttachedDocs: 5,
      dailyQuestionCap: 100,
    });
  });
});
