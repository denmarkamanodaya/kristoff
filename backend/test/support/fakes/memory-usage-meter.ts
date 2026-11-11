// In-memory UsageMeter; trusted because test/ports runs the same contract suite.
import { ZERO_USAGE, type Usage } from '../../../src/domain/model.js';
import type { UsageMeter } from '../../../src/ports/usage-meter.js';
import { appError } from '../../../src/shared/errors/app-error.js';
import { err, ok } from '../../../src/shared/result/result.js';

const add = (a: Usage, b: Usage): Usage => ({
  input: a.input + b.input,
  output: a.output + b.output,
  cacheRead: a.cacheRead + b.cacheRead,
  cacheWrite: a.cacheWrite + b.cacheWrite,
});

/** The fake plus its question counts and token counters. */
export const memoryUsageMeter = () => {
  const questions = new Map<string, number>();
  const tokens = new Map<string, Usage>();
  const meter: UsageMeter = {
    admitQuestion: (uid, day, cap) => {
      const count = questions.get(`${uid}/${day}`) ?? 0;
      if (count >= cap) return Promise.resolve(err(appError('DAILY_CAP_REACHED', 'cap')));
      questions.set(`${uid}/${day}`, count + 1);
      return Promise.resolve(ok(undefined));
    },
    addTokens: (uid, day, usage) => {
      for (const key of [`${uid}/${day}`, `${uid}/TOTAL`])
        tokens.set(key, add(tokens.get(key) ?? ZERO_USAGE, usage));
      return Promise.resolve(ok(undefined));
    },
    read: (uid, day) =>
      Promise.resolve(
        ok({
          today: tokens.get(`${uid}/${day}`) ?? ZERO_USAGE,
          total: tokens.get(`${uid}/TOTAL`) ?? ZERO_USAGE,
        }),
      ),
  };
  return { meter, questions, tokens };
};
