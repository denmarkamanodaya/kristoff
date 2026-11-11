// Domain value types (docs/contract.md "Shapes"). Ports and use cases speak only these. The wire
// schemas in src/contracts mirror them and a compile-time test pins that they stay assignable.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.

/** The verified caller: `uid` comes from the session token's `sub`, never from a request. */
export interface Principal {
  readonly uid: string;
}

/** Token counts of one model call or a running total (non-negative integers). */
export interface Usage {
  readonly input: number;
  readonly output: number;
  readonly cacheRead: number;
  readonly cacheWrite: number;
}

/** Upload state of a document: `pending` until the PDF is confirmed in the bucket. */
type DocStatus = 'pending' | 'ready';

/** A library document (a PDF in the docs bucket). `createdAt` is an ISO instant. */
export interface Doc {
  readonly id: string;
  readonly title: string;
  readonly size: number;
  readonly status: DocStatus;
  readonly createdAt: string;
}

/** A passage of an attached document the answer cites. Pages are 1-based and inclusive. */
export interface Citation {
  readonly n: number;
  readonly docId: string | null;
  readonly docTitle: string;
  readonly page: number | null;
  readonly endPage: number | null;
  readonly quote: string;
}

/** A chat as listed in the sidebar. `updatedAt` is an ISO instant. */
export interface ChatSummary {
  readonly id: string;
  readonly title: string;
  readonly docIds: readonly string[];
  readonly updatedAt: string;
}

/** A chat with its creation instant. */
export interface Chat extends ChatSummary {
  readonly createdAt: string;
}

/** Who wrote a message. */
export type Role = 'user' | 'assistant';

/** One stored chat message. `id` is a ULID, so ids sort by time. */
export interface Message {
  readonly id: string;
  readonly role: Role;
  readonly text: string;
  readonly citations: readonly Citation[];
  readonly usage: Usage | null;
  readonly model: string | null;
  readonly stopReason: string | null;
  readonly at: string;
}

/** A stored value with its optimistic-lock version. */
export interface Versioned<T> {
  readonly value: T;
  readonly version: number;
}

/** One page of a bounded list. `nextCursor` is opaque; `null` when there is no next page. */
export interface Page<T> {
  readonly items: readonly T[];
  readonly nextCursor: string | null;
}

/** A zero usage, the start of every counter. */
export const ZERO_USAGE: Usage = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
