//! Chat use cases: create, list, get, update and delete, plus the attached-docs check and the
//! walk over every chat that delete-doc and delete-project share. Ported from `create-chat`,
//! `list-chats`, `get-chat`, `update-chat`, `delete-chat`, `resolve-attached-docs`,
//! `load-attachments` and `each-owner-chat`.

use crate::domain::attachment::{attachment_target, parse_attachment_id};
use crate::domain::names::trimmed_name;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::limits::{CHAT_MESSAGES_MAX, CHATS_PAGE_SIZE, MAX_CHAT_TITLE, NEW_CHAT_TITLE};
use crate::model::{Chat, Doc, Message, Page};
use crate::ports::{
    ChatChanges, ChatPatch, ChatStore, Clock, DocStore, IdGenerator, ProjectChange, ProjectStore,
};

/// What the chat use cases need.
pub struct ChatDeps<'a> {
    /// Chats and messages.
    pub chats: &'a dyn ChatStore,
    /// Documents (the attached-docs check).
    pub docs: &'a dyn DocStore,
    /// Projects (moving a chat into one).
    pub projects: &'a dyn ProjectStore,
    /// Time.
    pub clock: &'a dyn Clock,
    /// Ids.
    pub ids: &'a dyn IdGenerator,
    /// Most attachments per chat (configuration).
    pub max_attached_docs: usize,
}

/// A chat with its newest messages, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatWithMessages {
    /// The chat.
    pub chat: Chat,
    /// Up to 200 messages.
    pub messages: Vec<Message>,
}

/// The ids of every chat `select` keeps, walking all pages newest first.
///
/// # Errors
/// The first store failure.
pub fn each_chat(chats: &dyn ChatStore, select: impl Fn(&Chat) -> bool) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = chats.list_chats(cursor.as_deref(), CHATS_PAGE_SIZE)?;
        out.extend(
            page.items
                .iter()
                .filter(|c| select(c))
                .map(|c| c.id.clone()),
        );
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return Ok(out),
        }
    }
}

/// Checks attachment ids: at most the configured cap, each an existing ready doc or part.
/// Returns them de-duplicated in request order.
///
/// # Errors
/// `VALIDATION_FAILED` over the cap; `DOC_NOT_FOUND` for an unusable id.
pub fn resolve_attached_docs(deps: &ChatDeps<'_>, doc_ids: &[String]) -> AppResult<Vec<String>> {
    let mut unique: Vec<String> = Vec::new();
    for id in doc_ids {
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    if unique.len() > deps.max_attached_docs {
        let message = format!("attach at most {} docs", deps.max_attached_docs);
        return Err(AppError::new(ErrorCode::ValidationFailed, message));
    }
    let refs: Vec<_> = unique.iter().map(|id| parse_attachment_id(id)).collect();
    let named: Vec<String> = refs.iter().flatten().map(|r| r.doc_id.clone()).collect();
    let found: Vec<Doc> = if named.is_empty() {
        vec![]
    } else {
        deps.docs.get_docs(&named)?
    };
    let usable = refs.iter().all(|r| {
        r.as_ref().is_some_and(|r| {
            attachment_target(r, found.iter().find(|d| d.id == r.doc_id)).is_some()
        })
    });
    if usable {
        Ok(unique)
    } else {
        Err(AppError::new(
            ErrorCode::DocNotFound,
            "a doc or part is missing or not ready",
        ))
    }
}

/// Creates a chat titled "New chat" with optional attachments.
///
/// # Errors
/// `VALIDATION_FAILED` / `DOC_NOT_FOUND` from the attachment check, and store failures.
pub fn create_chat(deps: &ChatDeps<'_>, doc_ids: &[String]) -> AppResult<Chat> {
    let doc_ids = resolve_attached_docs(deps, doc_ids)?;
    let now = deps.clock.now_ms();
    let chat = Chat {
        id: deps.ids.new_id(),
        title: NEW_CHAT_TITLE.to_owned(),
        doc_ids,
        pinned: false,
        project_id: None,
        created_at_ms: now,
        updated_at_ms: now,
    };
    deps.chats.create_chat(&chat)?;
    Ok(chat)
}

/// One page of up to 50 chats, newest `updated_at` first.
///
/// # Errors
/// `VALIDATION_FAILED` for a foreign cursor.
pub fn list_chats(deps: &ChatDeps<'_>, cursor: Option<&str>) -> AppResult<Page<Chat>> {
    deps.chats.list_chats(cursor, CHATS_PAGE_SIZE)
}

/// The chat plus its newest 200 messages, oldest first.
///
/// # Errors
/// `CHAT_NOT_FOUND`.
pub fn get_chat(deps: &ChatDeps<'_>, chat_id: &str) -> AppResult<ChatWithMessages> {
    let chat = deps.chats.get_chat(chat_id)?.value;
    let messages = deps.chats.recent_messages(chat_id, CHAT_MESSAGES_MAX)?;
    Ok(ChatWithMessages { chat, messages })
}

fn resolve_patch(deps: &ChatDeps<'_>, changes: &ChatChanges) -> AppResult<ChatPatch> {
    let title = changes
        .title
        .as_deref()
        .map(|t| trimmed_name(t, MAX_CHAT_TITLE, "title"));
    let title = title.transpose()?;
    if let Some(ProjectChange::MoveTo(project_id)) = &changes.project {
        deps.projects.get_project(project_id)?;
    }
    let doc_ids = changes
        .doc_ids
        .as_deref()
        .map(|ids| resolve_attached_docs(deps, ids));
    let changes = ChatChanges {
        title,
        doc_ids: doc_ids.transpose()?,
        pinned: changes.pinned,
        project: changes.project.clone(),
    };
    Ok(ChatPatch {
        changes,
        updated_at_ms: deps.clock.now_ms(),
    })
}

/// Renames, re-attaches, pins or moves a chat (at least one change; the request check enforces it).
///
/// # Errors
/// `CHAT_NOT_FOUND`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`, `PROJECT_NOT_FOUND`, `VERSION_CONFLICT`.
pub fn update_chat(deps: &ChatDeps<'_>, chat_id: &str, changes: &ChatChanges) -> AppResult<Chat> {
    let found = deps.chats.get_chat(chat_id)?;
    let patch = resolve_patch(deps, changes)?;
    deps.chats.update_chat(chat_id, &patch, found.version)
}

/// Deletes a chat: its messages first, the chat LAST, so a failure midway can be repeated.
///
/// # Errors
/// `CHAT_NOT_FOUND` and store failures.
pub fn delete_chat(deps: &ChatDeps<'_>, chat_id: &str) -> AppResult<()> {
    deps.chats.get_chat(chat_id)?;
    deps.chats.delete_messages(chat_id)?;
    deps.chats.delete_chat(chat_id)
}
