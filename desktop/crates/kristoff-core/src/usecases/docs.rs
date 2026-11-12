//! Library use cases: create (pending doc + upload URL), list, rename, link and delete. Ported from
//! `create-doc-upload`, `list-docs`, `rename-doc`, `get-doc-link` and `delete-doc`.

use crate::domain::attachment::is_attachment_of;
use crate::domain::links::{doc_link, upload_url};
use crate::domain::names::{title_of, trimmed_name};
use crate::error::{AppResult, ErrorCode};
use crate::limits::{DOC_LINK_TTL_SECONDS, DOCS_PAGE_SIZE, MAX_DOC_TITLE};
use crate::model::{Doc, DocStatus, Page};
use crate::ports::{
    ChatChanges, ChatPatch, ChatStore, Clock, DocPatch, DocStore, FileStore, IdGenerator,
};
use crate::usecases::chats::each_chat;
use crate::usecases::viewer::ready_doc;

/// What the library use cases need.
pub struct LibraryDeps<'a> {
    /// Documents.
    pub docs: &'a dyn DocStore,
    /// Chats (delete unlinks the doc from them).
    pub chats: &'a dyn ChatStore,
    /// PDF bytes.
    pub files: &'a dyn FileStore,
    /// Time.
    pub clock: &'a dyn Clock,
    /// Ids.
    pub ids: &'a dyn IdGenerator,
}

/// The upload request: original file name and declared size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocUploadInput {
    /// Original file name.
    pub file_name: String,
    /// Declared size in bytes.
    pub size: u64,
}

/// A pending doc and where its bytes go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocUpload {
    /// The pending doc.
    pub doc: Doc,
    /// `kristoff-upload://<docId>`.
    pub upload_url: String,
}

/// A link to a doc or a part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLink {
    /// `kristoff://doc/<id>[/part-<n>]`.
    pub url: String,
    /// When the link stops being offered, epoch ms.
    pub expires_at_ms: i64,
}

/// Records a pending doc and returns its upload URL.
///
/// # Errors
/// Store failures.
pub fn create_doc(deps: &LibraryDeps<'_>, input: &DocUploadInput) -> AppResult<DocUpload> {
    let doc = Doc {
        id: deps.ids.new_id(),
        title: title_of(&input.file_name),
        size: input.size,
        status: DocStatus::Pending,
        created_at_ms: deps.clock.now_ms(),
        page_count: None,
        parts: vec![],
    };
    deps.docs.create_doc(&doc)?;
    let url = upload_url(&doc.id);
    Ok(DocUpload {
        doc,
        upload_url: url,
    })
}

/// One page of up to 100 docs, newest first.
///
/// # Errors
/// `VALIDATION_FAILED` for a foreign cursor.
pub fn list_docs(deps: &LibraryDeps<'_>, cursor: Option<&str>) -> AppResult<Page<Doc>> {
    deps.docs.list_docs(cursor, DOCS_PAGE_SIZE)
}

/// Renames a doc (title trimmed, 1..200).
///
/// # Errors
/// `VALIDATION_FAILED` (blank), `DOC_NOT_FOUND`, `VERSION_CONFLICT`.
pub fn rename_doc(deps: &LibraryDeps<'_>, doc_id: &str, title: &str) -> AppResult<Doc> {
    let title = trimmed_name(title, MAX_DOC_TITLE, "title")?;
    let found = deps.docs.get_doc(doc_id)?;
    let patch = DocPatch {
        title: Some(title),
        ..DocPatch::default()
    };
    deps.docs.update_doc(doc_id, &patch, found.version)
}

/// The link of a ready doc, or of one of its parts.
///
/// # Errors
/// `DOC_NOT_FOUND` when missing, not ready, or the part does not exist.
pub fn get_doc_link(deps: &LibraryDeps<'_>, doc_id: &str, part: Option<u32>) -> AppResult<DocLink> {
    ready_doc(deps.docs, doc_id, part)?;
    let expires_at_ms = deps.clock.now_ms() + DOC_LINK_TTL_SECONDS * 1000;
    Ok(DocLink {
        url: doc_link(doc_id, part),
        expires_at_ms,
    })
}

/// Removes the doc's attachment ids from one chat at the version just read (`updated_at` kept, so
/// the chat does not jump to the top). A chat deleted meanwhile is skipped.
fn unlink_from_chat(deps: &LibraryDeps<'_>, chat_id: &str, doc_id: &str) -> AppResult<()> {
    let chat = match deps.chats.get_chat(chat_id) {
        Ok(chat) => chat,
        Err(e) if e.code == ErrorCode::ChatNotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let kept = chat
        .value
        .doc_ids
        .iter()
        .filter(|id| !is_attachment_of(id, doc_id))
        .cloned();
    let changes = ChatChanges {
        doc_ids: Some(kept.collect()),
        ..ChatChanges::default()
    };
    let patch = ChatPatch {
        changes,
        updated_at_ms: chat.value.updated_at_ms,
    };
    deps.chats
        .update_chat(chat_id, &patch, chat.version)
        .map(|_| ())
}

/// Deletes a doc: unlinks it (and its parts) from every chat, purges its files, then removes the
/// doc LAST, so a failure midway can simply be repeated.
///
/// # Errors
/// `DOC_NOT_FOUND`, `VERSION_CONFLICT` and store failures (the doc then still exists).
pub fn delete_doc(deps: &LibraryDeps<'_>, doc_id: &str) -> AppResult<()> {
    deps.docs.get_doc(doc_id)?;
    let attached = each_chat(deps.chats, |c| {
        c.doc_ids.iter().any(|id| is_attachment_of(id, doc_id))
    })?;
    for chat_id in attached {
        unlink_from_chat(deps, &chat_id, doc_id)?;
    }
    deps.files.purge(doc_id)?;
    deps.docs.remove_doc(doc_id)
}
