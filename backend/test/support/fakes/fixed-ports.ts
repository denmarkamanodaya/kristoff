// Small fakes: flags, secrets and a scripted answer model.
import type {
  AnswerEnd,
  AnswerEvent,
  ModelRequest,
} from '../../../src/domain/chats/model-request.js';
import type { AnswerModel } from '../../../src/ports/answer-model.js';
import type { FlagReader } from '../../../src/ports/flag-reader.js';
import type { SecretName, SecretSource } from '../../../src/ports/secret-source.js';
import { appError, type AppError } from '../../../src/shared/errors/app-error.js';
import { err, ok, type Result } from '../../../src/shared/result/result.js';

/** A flag reader answering `value` (or the given error). */
export const fixedFlags = (value: Result<boolean, AppError> = ok(true)): FlagReader => ({
  isAiEnabled: () => Promise.resolve(value),
});

/** A secret source over a map; missing names fail like the real one. */
export const fixedSecrets = (values: Partial<Record<SecretName, string>>): SecretSource => ({
  get: (name) => {
    const value = values[name];
    return Promise.resolve(
      value ? ok(value) : err(appError('INTERNAL', `secret ${name} unavailable`)),
    );
  },
});

/** A model that replays `events` then ends with `end`; records each request. */
export const scriptedModel = (events: readonly AnswerEvent[], end: Result<AnswerEnd, AppError>) => {
  const requests: ModelRequest[] = [];
  const model: AnswerModel = {
    stream: (request, onEvent) => {
      requests.push(request);
      events.forEach(onEvent);
      return Promise.resolve(end);
    },
  };
  return { model, requests };
};
