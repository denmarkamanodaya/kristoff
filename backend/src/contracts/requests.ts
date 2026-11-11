// Wire request shapes (docs/contract.md "Routes", "Stream"): strict objects, every field bounded.
// Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
import { z } from 'zod';
import {
  MAX_CHAT_TITLE,
  MAX_DOC_IDS_CEILING,
  MAX_DOC_TITLE,
  MAX_FILE_NAME,
  MAX_QUESTION,
  MAX_UPLOAD_BYTES,
} from '../domain/limits.js';
import { Cursor, Id } from './shapes.js';

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
  .object({ docIds: z.array(Id).max(MAX_DOC_IDS_CEILING).optional() })
  .strict();

/** `PATCH /v1/chats/{id}` body: at least one field. */
export const UpdateChatRequest = z
  .object({
    title: z.string().min(1).max(MAX_CHAT_TITLE).optional(),
    docIds: z.array(Id).max(MAX_DOC_IDS_CEILING).optional(),
  })
  .strict()
  .refine((body) => body.title !== undefined || body.docIds !== undefined, {
    message: 'send title or docIds',
  });

/** Stream `POST /v1/chats/{id}/ask` body. */
export const AskRequest = z.object({ text: z.string().min(1).max(MAX_QUESTION) }).strict();
