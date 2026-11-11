// AnswerSink: where ask-question pushes a streamed answer (the stream handler implements it as
// SSE). Contract: docs/contract.md "Stream". `open` marks the point after which failures are sent
// as an `error` event instead of a JSON error status.
import type { Citation } from '../domain/model.js';

/** Receives one answer as it streams. */
export interface AnswerSink {
  /** Called once, after every pre-stream check passed and before the first event. */
  open(): void;
  /** A piece of answer text. */
  text(text: string): void;
  /** A numbered citation (the client appends ` [n]` to the text, as the stored text has). */
  citation(citation: Citation): void;
}
