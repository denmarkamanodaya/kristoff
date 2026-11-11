// The contract's fixed limits (docs/contract.md "Routes", "Shapes", "Auth flow"). One source: the
// Zod contracts and the use cases both import these. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
// Configurable limits (attached docs, history turns, daily cap) come from env, not from here.

/** Largest PDF upload in bytes (32 MiB). */
export const MAX_UPLOAD_BYTES = 33_554_432;
/** Longest uploaded file name in characters. */
export const MAX_FILE_NAME = 255;
/** Longest document title in characters. */
export const MAX_DOC_TITLE = 200;
/** Longest chat title in characters. */
export const MAX_CHAT_TITLE = 120;
/** Longest question text in characters. */
export const MAX_QUESTION = 8000;
/** Longest stored message text in characters. */
export const MAX_MESSAGE_TEXT = 200_000;
/** Most citations stored on one message. */
export const MAX_CITATIONS = 500;
/** Longest cited quote in characters. */
export const MAX_QUOTE = 4000;
/** Longest opaque list cursor in characters. */
export const MAX_CURSOR = 512;
/** Hard ceiling on `docIds` in a request; the configured `MAX_ATTACHED_DOCS` is checked below it. */
export const MAX_DOC_IDS_CEILING = 20;
/** Library page size. */
export const DOCS_PAGE_SIZE = 100;
/** Chat list page size. */
export const CHATS_PAGE_SIZE = 50;
/** Most messages returned with a chat (the newest ones, oldest first). */
export const CHAT_MESSAGES_MAX = 200;
/** Lifetime of the presigned PDF link the web opens, in seconds. */
export const DOC_LINK_TTL_SECONDS = 300;
/** Lifetime of the presigned upload URL, in seconds. */
export const UPLOAD_URL_TTL_SECONDS = 300;
/** Lifetime of the presigned GET handed to the model as a document source, in seconds. */
export const MODEL_DOC_URL_TTL_SECONDS = 900;
/** Title a new chat gets until its first question renames it. */
export const NEW_CHAT_TITLE = 'New chat';
/** Characters of the first question used as the chat title. */
export const AUTO_TITLE_CHARS = 60;
/** The only content type the docs bucket accepts. */
export const PDF_CONTENT_TYPE = 'application/pdf';
/** The magic bytes every PDF starts with. */
export const PDF_MAGIC = '%PDF-';
