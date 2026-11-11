// FollowUpSuggester port: suggests next questions after an answer (docs/contract.md "v2 Answer
// extras"). It never fails: any problem gives no questions (the adapter logs a warn).
import type { FollowUps } from '../domain/model.js';

/** Suggests follow-up questions. */
export interface FollowUpSuggester {
  /** Up to 5 questions (each 1..120 chars); `{ questions: [], usage }` on any failure. */
  suggest(input: { readonly question: string; readonly answerExcerpt: string }): Promise<FollowUps>;
}
