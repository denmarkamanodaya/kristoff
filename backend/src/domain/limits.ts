// The contract's fixed limits (docs/contract.md "Routes", "Shapes", "Auth flow"). One source: the
// Zod contracts and the use cases both import these. Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md.
// Configurable limits (attached docs, history turns, daily cap) come from env, not from here.

/** Largest PDF upload in bytes (200 MiB; docs/contract.md "v2 Large PDFs"). */
export const MAX_UPLOAD_BYTES = 209_715_200;
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
/** Longest project name in characters. */
export const MAX_PROJECT_NAME = 80;
/** Most projects one owner may have. */
export const MAX_PROJECTS = 50;
/** Most follow-up questions stored per answer. */
export const MAX_FOLLOW_UPS = 5;
/** Longest follow-up question in characters. */
export const MAX_FOLLOW_UP = 120;
/** Characters of the answer the follow-up call sees. */
export const FOLLOW_UP_ANSWER_CHARS = 2000;
/** Most pages in one model document (whole doc or part). */
export const PART_MAX_PAGES = 100;
/** Largest model document (whole doc or part) in bytes (30 MiB). */
export const PART_MAX_BYTES = 31_457_280;
/** Most parts one document may have (bounds the stored list). */
export const MAX_PARTS = 200;
