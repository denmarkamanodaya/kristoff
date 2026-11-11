import { describe, expect, it } from 'vitest';
import { getConfig } from './v1.js';

describe('get-config v1', () => {
  it('returns exactly the configured values', () => {
    const config = { model: 'claude-sonnet-5-5', maxAttachedDocs: 5, dailyQuestionCap: 100 };

    expect(getConfig(config)()).toEqual(config);
  });
});
