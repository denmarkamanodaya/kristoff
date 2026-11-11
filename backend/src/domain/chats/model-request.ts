// Domain shapes of a model call (no SDK types): the AnswerModel port maps them to the vendor
// request. Contract: docs/contract.md "Model call". Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import type { Role, Usage } from '../model.js';

/** A ready document handed to the model by a short-lived presigned URL. */
export interface AttachedDoc {
  /** The library doc id (also for a part). */
  readonly id: string;
  /** The document title the model sees (a part's title names its page range). */
  readonly title: string;
  /** The library title, used on citations. */
  readonly libraryTitle: string;
  readonly url: string;
  /** Added to a cited page to get the original page (0 for a whole doc, startPage − 1 for a part). */
  readonly pageOffset: number;
}

/** One content block of a turn: a cited PDF, or plain text. */
export type TurnBlock =
  | {
      readonly kind: 'doc';
      readonly title: string;
      readonly url: string;
      /** True on the last document only: the prompt-cache breakpoint. */
      readonly isCacheBreakpoint: boolean;
    }
  | { readonly kind: 'text'; readonly text: string };

/** One conversation turn. */
export interface ModelTurn {
  readonly role: Role;
  readonly blocks: readonly TurnBlock[];
}

/** Everything one model call needs. Prompt and parameters come from src/contracts/model-config. */
export interface ModelRequest {
  readonly model: string;
  readonly maxTokens: number;
  readonly turns: readonly ModelTurn[];
}

/** A citation as the model reports it, before it is numbered and tied to a library doc. */
export interface CitationSource {
  /** 0-based index of the cited document in the request's document order. */
  readonly documentIndex: number;
  readonly documentTitle: string | null;
  /** 1-based first page, or null when the location is not a page range. */
  readonly startPage: number | null;
  /** Exclusive end page as the API reports it, or null. */
  readonly endPageExclusive: number | null;
  readonly citedText: string;
}

/** One streamed event of an answer. */
export type AnswerEvent =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'citation'; readonly source: CitationSource };

/** How a successful model call ended. */
export interface AnswerEnd {
  readonly usage: Usage;
  readonly stopReason: string | null;
}
