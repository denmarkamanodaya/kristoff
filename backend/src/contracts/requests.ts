// Wire request shapes (docs/contract.md "Routes", "Stream"): strict objects, every field bounded.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';
import {
  MAX_CHAT_TITLE,
  MAX_DOC_IDS_CEILING,
  MAX_DOC_TITLE,
  MAX_FILE_NAME,
  MAX_PROJECT_NAME,
  MAX_QUESTION,
  MAX_UPLOAD_BYTES,
} from '../domain/limits.js';
import { AttachmentId, Cursor, Id } from './shapes.js';

/** Largest JSON body any route accepts, in UTF-8 bytes (the 8000-char question fits). */
export const MAX_BODY_BYTES = 64 * 1024;

/** `POST /v1/auth/verify` body. */
export const VerifyRequest = z
  .object({
    code: z
      .string()
      .length(6)
      .regex(/^\d{6}$/),
  })
  .strict();

/** `?cursor` of the list routes. */
export const ListQuery = z.object({ cursor: Cursor.optional() }).strict();

/** `{id}` path parameter. */
export const IdPath = z.object({ id: Id }).strict();

/** `POST /v1/docs` body. */
export const CreateDocRequest = z
  .object({
    fileName: z.string().min(1).max(MAX_FILE_NAME),
    size: z.number().int().min(1).max(MAX_UPLOAD_BYTES),
  })
  .strict();

/** `PATCH /v1/docs/{id}` body. */
export const RenameDocRequest = z.object({ title: z.string().min(1).max(MAX_DOC_TITLE) }).strict();

/** `POST /v1/chats` body; the configured `MAX_ATTACHED_DOCS` is checked by the use case. */
export const CreateChatRequest = z
  .object({ docIds: z.array(AttachmentId).max(MAX_DOC_IDS_CEILING).optional() })
  .strict();

/** `PATCH /v1/chats/{id}` body: at least one field; `projectId: null` ungroups the chat. */
export const UpdateChatRequest = z
  .object({
    title: z.string().min(1).max(MAX_CHAT_TITLE).optional(),
    docIds: z.array(AttachmentId).max(MAX_DOC_IDS_CEILING).optional(),
    pinned: z.boolean().optional(),
    projectId: Id.nullable().optional(),
  })
  .strict()
  .refine((body) => Object.values(body).some((v) => v !== undefined), {
    message: 'send at least one of title, docIds, pinned, projectId',
  });

/** Stream `POST /v1/chats/{id}/ask` body; `model` must be in `MODEL_IDS` (checked by the use case). */
export const AskRequest = z
  .object({
    text: z.string().min(1).max(MAX_QUESTION),
    model: z.string().min(1).max(64).optional(),
  })
  .strict();

/** `POST /v1/projects` and `PATCH /v1/projects/{id}` body. */
export const ProjectNameRequest = z
  .object({ name: z.string().min(1).max(MAX_PROJECT_NAME) })
  .strict();

/** `GET /v1/docs/{id}/link` query: optional `part` (1..999) links one part of a large doc. */
export const LinkQuery = z
  .object({
    part: z
      .string()
      .max(3)
      .regex(/^[1-9][0-9]{0,2}$/)
      .optional(),
  })
  .strict();

/** The docs-split async event (sent by confirm through DocSplitTrigger). */
export const SplitEvent = z
  .object({
    uid: z
      .string()
      .max(64)
      .regex(/^[A-Za-z0-9_-]{1,64}$/),
    docId: Id,
  })
  .strict();
