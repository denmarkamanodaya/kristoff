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

/**
 * Upload state of a document: `pending` until the PDF is confirmed, `processing` while docs-split
 * counts its pages and writes parts, then `ready` or `failed`.
 */
export type DocStatus = 'pending' | 'processing' | 'ready' | 'failed';

/** One part of a large document: 1-based inclusive original page range and size in bytes. */
export interface DocPart {
  readonly n: number;
  readonly startPage: number;
  readonly endPage: number;
  readonly size: number;
}

/** A library document (a PDF in the docs bucket). `createdAt` is an ISO instant. */
export interface Doc {
  readonly id: string;
  readonly title: string;
  readonly size: number;
  readonly status: DocStatus;
  readonly createdAt: string;
  /** Pages in the PDF, known once processed; null before. */
  readonly pageCount: number | null;
  /** Parts of a large PDF (empty for a document attached whole). */
  readonly parts: readonly DocPart[];
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
  /** Shown in the Pinned group. */
  readonly pinned: boolean;
  /** The project the chat is grouped under, or null. */
  readonly projectId: string | null;
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
  /** Assistant: ms from request start to the final event; null for user messages. */
  readonly durationMs: number | null;
  /** Assistant: up to 5 suggested next questions (each 1..120 chars); [] for user messages. */
  readonly followUps: readonly string[];
}

/** Suggested next questions and the tokens the suggestion call used (null when it failed). */
export interface FollowUps {
  readonly questions: readonly string[];
  readonly usage: Usage | null;
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

/** A named group of chats. Instants are ISO. */
export interface Project {
  readonly id: string;
  readonly name: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

/** A zero usage, the start of every counter. */
export const ZERO_USAGE: Usage = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
