//! The contract's fixed limits, the same values as `backend/src/domain/limits.ts` (a contracts
//! test reads that file and pins every shared value). Desktop-only limits are marked.

/// Largest PDF upload in bytes (200 MiB).
pub const MAX_UPLOAD_BYTES: u64 = 209_715_200;
/// Longest uploaded file name in UTF-16 units.
pub const MAX_FILE_NAME: usize = 255;
/// Longest document title in UTF-16 units.
pub const MAX_DOC_TITLE: usize = 200;
/// Longest chat title in UTF-16 units.
pub const MAX_CHAT_TITLE: usize = 120;
/// Longest question text in UTF-16 units.
pub const MAX_QUESTION: usize = 8000;
/// Longest opaque list cursor in characters.
pub const MAX_CURSOR: usize = 512;
/// Hard ceiling on `docIds` in a request; the configured attach cap is checked below it.
pub const MAX_DOC_IDS_CEILING: usize = 20;
/// Library page size.
pub const DOCS_PAGE_SIZE: usize = 100;
/// Chat list page size.
pub const CHATS_PAGE_SIZE: usize = 50;
/// Most messages returned with a chat (the newest ones, oldest first).
pub const CHAT_MESSAGES_MAX: usize = 200;
/// Lifetime of a document link, in seconds.
pub const DOC_LINK_TTL_SECONDS: i64 = 300;
/// Title a new chat gets until its first question renames it.
pub const NEW_CHAT_TITLE: &str = "New chat";
/// The only content type an upload is declared with.
pub const PDF_CONTENT_TYPE: &str = "application/pdf";
/// The magic bytes every PDF starts with.
pub const PDF_MAGIC: &str = "%PDF-";
/// Longest project name in UTF-16 units.
pub const MAX_PROJECT_NAME: usize = 80;
/// Most projects the owner may have.
pub const MAX_PROJECTS: usize = 50;
/// Most parts one document may have.
pub const MAX_PARTS: usize = 200;
/// Desktop only: largest raw upload chunk accepted, in bytes (the web sends 8 MiB chunks).
pub const MAX_UPLOAD_CHUNK_BYTES: usize = 16 * 1024 * 1024;
/// Most pages of a PDF sent whole, and of each part.
pub const PART_MAX_PAGES: u32 = 100;
/// Most bytes of a PDF sent whole, and of each part (30 MiB).
pub const PART_MAX_BYTES: u64 = 31_457_280;
/// Longest stored message text in UTF-16 units.
pub const MAX_MESSAGE_TEXT: usize = 200_000;
/// Most citations kept per answer.
pub const MAX_CITATIONS: usize = 500;
/// Longest stored citation quote in UTF-16 units.
pub const MAX_QUOTE: usize = 4000;
/// Characters of the first question that become the chat title.
pub const AUTO_TITLE_CHARS: usize = 60;
/// Most follow-up questions kept.
pub const MAX_FOLLOW_UPS: usize = 5;
/// Longest follow-up question in UTF-16 units.
pub const MAX_FOLLOW_UP: usize = 120;
/// Characters of the answer the follow-up call sees.
pub const FOLLOW_UP_ANSWER_CHARS: usize = 2000;
/// Desktop only: most past messages sent to the model (the web had this as env `MAX_HISTORY_TURNS`).
pub const MAX_HISTORY_TURNS: usize = 20;
