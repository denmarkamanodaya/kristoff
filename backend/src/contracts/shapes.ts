// Wire response shapes (docs/contract.md "Shapes"). Handlers' tests parse every response with
// these; shapes.test.ts pins that the domain types stay assignable to them.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';
import {
  MAX_CHAT_TITLE,
  MAX_CITATIONS,
  MAX_CURSOR,
  MAX_DOC_IDS_CEILING,
  MAX_DOC_TITLE,
  MAX_FOLLOW_UP,
  MAX_FOLLOW_UPS,
  MAX_PROJECT_NAME,
  MAX_PROJECTS,
  MAX_MESSAGE_TEXT,
  MAX_PARTS,
  MAX_QUOTE,
  MAX_UPLOAD_BYTES,
} from '../domain/limits.js';

/** A 16-lowercase-hex id. */
export const Id = z
  .string()
  .length(16)
  .regex(/^[a-f0-9]{16}$/);
/** An ISO-8601 UTC instant with milliseconds and `Z`. */
const Instant = z.iso.datetime({ precision: 3 }).max(24);
const count = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
/** The opaque list cursor clients echo. */
export const Cursor = z.string().min(1).max(MAX_CURSOR);
const NextCursor = Cursor.nullable();
const Url = z.url().max(4096);

/** Token counters. */
export const UsageWire = z
  .object({ input: count, output: count, cacheRead: count, cacheWrite: count })
  .strict();

/** One part of a large document (original 1-based page range, size in bytes). */
const DocPartWire = z
  .object({
    n: z.number().int().min(1).max(MAX_PARTS),
    startPage: z.number().int().min(1).max(1_000_000),
    endPage: z.number().int().min(1).max(1_000_000),
    size: z.number().int().min(1).max(MAX_UPLOAD_BYTES),
  })
  .strict();

/** An entry of a chat's `docIds`: `<docId>` (whole doc) or `<docId>:p<n>` (one part). */
export const AttachmentId = z
  .string()
  .max(21)
  .regex(/^[a-f0-9]{16}(:p[0-9]{1,3})?$/);

/** A library document. */
export const DocWire = z
  .object({
    id: Id,
    title: z.string().max(MAX_DOC_TITLE),
    size: z.number().int().min(1).max(MAX_UPLOAD_BYTES),
    status: z.enum(['pending', 'processing', 'ready', 'failed']),
    createdAt: Instant,
    pageCount: z.number().int().min(1).max(1_000_000).nullable(),
    parts: z.array(DocPartWire).max(MAX_PARTS),
  })
  .strict();

/** A cited passage. */
export const CitationWire = z
  .object({
    n: z.number().int().min(1).max(MAX_CITATIONS),
    docId: Id.nullable(),
    docTitle: z.string().max(MAX_DOC_TITLE),
    page: z.number().int().min(0).max(1_000_000).nullable(),
    endPage: z.number().int().min(0).max(1_000_000).nullable(),
    quote: z.string().max(MAX_QUOTE),
  })
  .strict();

/** A chat as listed. */
export const ChatSummaryWire = z
  .object({
    id: Id,
    title: z.string().max(MAX_CHAT_TITLE),
    docIds: z.array(AttachmentId).max(MAX_DOC_IDS_CEILING),
    updatedAt: Instant,
    pinned: z.boolean(),
    projectId: Id.nullable(),
  })
  .strict();

/** A chat with its creation instant. */
export const ChatWire = ChatSummaryWire.extend({ createdAt: Instant }).strict();

/** One suggested follow-up question. */
const FollowUpText = z.string().min(1).max(MAX_FOLLOW_UP);

/** A stored message. */
export const MessageWire = z
  .object({
    id: z
      .string()
      .length(26)
      .regex(/^[0-9A-HJKMNP-TV-Z]{26}$/),
    role: z.enum(['user', 'assistant']),
    text: z.string().max(MAX_MESSAGE_TEXT),
    citations: z.array(CitationWire).max(MAX_CITATIONS),
    usage: UsageWire.nullable(),
    model: z.string().max(128).nullable(),
    stopReason: z.string().max(64).nullable(),
    at: Instant,
    durationMs: z.number().int().min(0).max(3_600_000).nullable(),
    followUps: z.array(FollowUpText).max(MAX_FOLLOW_UPS),
  })
  .strict();

/** `POST /v1/auth/verify` 200. */
export const SessionResponse = z
  .object({ token: z.string().max(1200), expiresAt: Instant })
  .strict();
/** `GET /v1/config` 200. */
export const ConfigResponse = z
  .object({
    model: z.string().max(64),
    models: z
      .array(z.object({ id: z.string().max(64), label: z.string().max(64) }).strict())
      .min(1)
      .max(5),
    maxAttachedDocs: z.number().int().min(0).max(MAX_DOC_IDS_CEILING),
    dailyQuestionCap: z.number().int().min(0).max(100_000),
  })
  .strict();
/** `GET /v1/docs` 200. */
export const DocsPageResponse = z
  .object({ items: z.array(DocWire).max(100), nextCursor: NextCursor })
  .strict();
/** `POST /v1/docs` 200. */
export const DocUploadResponse = z
  .object({
    doc: DocWire,
    uploadUrl: Url,
    uploadHeaders: z.object({ 'Content-Type': z.literal('application/pdf') }).strict(),
  })
  .strict();
/** `POST /v1/docs/{id}/confirm` and `PATCH /v1/docs/{id}` 200. */
export const DocResponse = z.object({ doc: DocWire }).strict();
/** `GET /v1/docs/{id}/link` 200. */
export const LinkResponse = z.object({ url: Url, expiresAt: Instant }).strict();
/** `GET /v1/chats` 200. */
export const ChatsPageResponse = z
  .object({ items: z.array(ChatSummaryWire).max(50), nextCursor: NextCursor })
  .strict();
/** `POST /v1/chats` and `PATCH /v1/chats/{id}` 200. */
export const ChatResponse = z.object({ chat: ChatWire }).strict();
/** `GET /v1/chats/{id}` 200. */
export const ChatWithMessagesResponse = z
  .object({ chat: ChatWire, messages: z.array(MessageWire).max(200) })
  .strict();
/** `GET /v1/usage` 200. */
export const UsageResponse = z.object({ today: UsageWire, total: UsageWire }).strict();
/** Stream `text` event data. */
export const TextEvent = z.object({ text: z.string().max(MAX_MESSAGE_TEXT) }).strict();
/** Stream `done` event data. */
export const DoneEvent = z.object({ message: MessageWire, usage: UsageWire }).strict();
/** `DELETE /v1/chats/{id}` 200. */
export const DeletedResponse = z.object({ deleted: z.literal(true) }).strict();
/** A project. */
export const ProjectWire = z
  .object({
    id: Id,
    name: z.string().max(MAX_PROJECT_NAME),
    createdAt: Instant,
    updatedAt: Instant,
  })
  .strict();
/** `GET /v1/projects` 200. */
export const ProjectsResponse = z
  .object({ items: z.array(ProjectWire).max(MAX_PROJECTS) })
  .strict();
/** `POST /v1/projects` and `PATCH /v1/projects/{id}` 200. */
export const ProjectResponse = z.object({ project: ProjectWire }).strict();
/** Stream `followups` event data. */
export const FollowUpsEvent = z
  .object({ followUps: z.array(FollowUpText).max(MAX_FOLLOW_UPS) })
  .strict();
