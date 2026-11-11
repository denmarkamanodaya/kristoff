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
  status: z.enum(['pending', 'ready']),
  createdAt: str,
});

/** A Chat item. */
export const ChatRow = z.object({
  ...Envelope,
  SK: str,
  title: str,
  docIds: z.array(str).max(100),
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
