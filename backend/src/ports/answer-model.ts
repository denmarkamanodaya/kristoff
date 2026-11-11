// AnswerModel port: one streamed model call in domain types (no SDK types cross it).
// Contract: docs/contract.md "Model call"; stack/ai-features.md.
import type { AppError } from '../shared/errors/app-error.js';
import type { Result } from '../shared/result/result.js';
import type { AnswerEnd, AnswerEvent, ModelRequest } from '../domain/chats/model-request.js';

/** Streams an answer. */
export interface AnswerModel {
  /**
   * Calls the model, passing each text and citation event to `onEvent` in order.
   * @returns Usage and stop reason, or `UPSTREAM_UNAVAILABLE` when the call fails at any point
   *   (events already delivered stay delivered).
   */
  stream(
    request: ModelRequest,
    onEvent: (event: AnswerEvent) => void,
  ): Promise<Result<AnswerEnd, AppError>>;
}
