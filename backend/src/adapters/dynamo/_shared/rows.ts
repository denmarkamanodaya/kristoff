// Stored row shapes (docs/contract.md "Storage"): parsed on every read so a malformed item fails
// closed instead of reaching the core. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';

const str = z.string().max(400_000);
const int = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
const nullableInt = int.nullable();

/** Fields every item carries. */
const Envelope = { schemaVersion: z.literal(1), version: int };

/** A Doc item. */
export const DocRow = z.object({
  ...Envelope,
  SK: str,
  title: str,
  size: int,
  status: z.enum(['pending', 'processing', 'ready', 'failed']),
  createdAt: str,
  // Added in v2 (docs written before have neither).
  pageCount: nullableInt.default(null),
  parts: z
    .array(z.object({ n: int, startPage: int, endPage: int, size: int }))
    .max(1000)
    .default([]),
});

/** A Chat item. */
export const ChatRow = z.object({
  ...Envelope,
  SK: str,
  title: str,
  docIds: z.array(str).max(100),
  createdAt: str,
  updatedAt: str,
  // Added in v2 without a schemaVersion bump: items written before have neither (defaults apply).
  pinned: z.boolean().default(false),
  projectId: str.nullable().default(null),
});

/** A Project item. */
export const ProjectRow = z.object({
  ...Envelope,
  SK: str,
  name: str,
  createdAt: str,
  updatedAt: str,
});

const UsageMap = z.object({ input: int, output: int, cacheRead: int, cacheWrite: int });

/** A Message item. */
export const MessageRow = z.object({
  ...Envelope,
  id: str,
  role: z.enum(['user', 'assistant']),
  text: z.string().max(400_000),
  citations: z
    .array(
      z.object({
        n: int,
        docId: str.nullable(),
        docTitle: str,
        page: nullableInt,
        endPage: nullableInt,
        quote: str,
      }),
    )
    .max(1000),
  usage: UsageMap.nullable(),
  model: str.nullable(),
  stopReason: str.nullable(),
  at: str,
  // Added in v2 (messages written before have neither).
  durationMs: nullableInt.default(null),
  followUps: z.array(str).max(10).default([]),
});

/** The TOTP state item. */
export const TotpRow = z.object({
  ...Envelope,
  lastStep: int,
  failCount: int,
  lockedUntil: int,
});

/** A usage counter item (day or total); counters absent until first added. */
export const UsageRow = z.object({
  SK: str,
  input: int.optional(),
  output: int.optional(),
  cacheRead: int.optional(),
  cacheWrite: int.optional(),
});
