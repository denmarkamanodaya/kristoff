//! Chat commands: `list_chats`, `create_chat`, `get_chat`, `update_chat`, `delete_chat`, plus
//! `ask` (Phase 1: `AI_DISABLED`) and `cancel_ask`.

use super::{CommandResult, Shared, blocking, unlocked};
use crate::composition::Composition;
use kristoff_contracts::envelopes::{
    ChatResponse, ChatWithMessagesResponse, ChatsPageResponse, DeletedResponse,
};
use kristoff_contracts::requests::{self, ChatPatchRequest};
use kristoff_contracts::shapes::{ChatDto, ChatSummaryDto, MessageDto};
use kristoff_core::error::{AppError, ErrorCode};
use kristoff_core::usecases::chats;
use tauri::State;
use tauri::ipc::Channel;

/// One page of chats.
///
/// # Errors
/// `UNAUTHENTICATED`; `VALIDATION_FAILED` for a bad cursor.
pub fn list(app: &Composition, cursor: Option<&str>) -> CommandResult<ChatsPageResponse> {
    unlocked(app)?;
    let page = chats::list_chats(&app.chats(), requests::cursor(cursor)?)?;
    let items = page.items.iter().map(ChatSummaryDto::from).collect();
    Ok(ChatsPageResponse {
        items,
        next_cursor: page.next_cursor,
    })
}

/// Creates a chat with optional attachments.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `DOC_NOT_FOUND`.
pub fn create(app: &Composition, doc_ids: Option<&[String]>) -> CommandResult<ChatResponse> {
    unlocked(app)?;
    let doc_ids = requests::doc_ids(doc_ids.unwrap_or_default())?;
    let chat = chats::create_chat(&app.chats(), doc_ids)?;
    Ok(ChatResponse {
        chat: ChatDto::from(&chat),
    })
}

/// A chat and its newest messages.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `CHAT_NOT_FOUND`.
pub fn get(app: &Composition, chat_id: &str) -> CommandResult<ChatWithMessagesResponse> {
    unlocked(app)?;
    let found = chats::get_chat(&app.chats(), requests::id(chat_id)?)?;
    let messages = found.messages.iter().map(MessageDto::from).collect();
    Ok(ChatWithMessagesResponse {
        chat: ChatDto::from(&found.chat),
        messages,
    })
}

/// Applies a patch `{title?, docIds?, pinned?, projectId?}`.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `CHAT_NOT_FOUND`, `DOC_NOT_FOUND`, `PROJECT_NOT_FOUND`.
pub fn update(
    app: &Composition,
    chat_id: &str,
    patch: serde_json::Value,
) -> CommandResult<ChatResponse> {
    unlocked(app)?;
    let chat_id = requests::id(chat_id)?;
    let changes = ChatPatchRequest::parse(patch)?;
    let chat = chats::update_chat(&app.chats(), chat_id, &changes)?;
    Ok(ChatResponse {
        chat: ChatDto::from(&chat),
    })
}

/// Deletes a chat and its messages.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `CHAT_NOT_FOUND`.
pub fn delete(app: &Composition, chat_id: &str) -> CommandResult<DeletedResponse> {
    unlocked(app)?;
    chats::delete_chat(&app.chats(), requests::id(chat_id)?)?;
    Ok(DeletedResponse { deleted: true })
}

/// Phase 1: asking is not built yet, so every valid question answers `AI_DISABLED`.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, then always `AI_DISABLED` (no `retryAfter`).
pub fn ask_now(
    app: &Composition,
    chat_id: &str,
    text: &str,
    model: Option<&str>,
) -> CommandResult<()> {
    unlocked(app)?;
    requests::id(chat_id)?;
    requests::ask(text, model)?;
    Err(AppError::new(ErrorCode::AiDisabled, "asking arrives in the next build").into())
}

/// Cancels a running answer (Phase 1: none can run).
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`.
pub fn cancel(app: &Composition, chat_id: &str) -> CommandResult<()> {
    unlocked(app)?;
    requests::id(chat_id)?;
    Ok(())
}

/// `list_chats {cursor}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn list_chats(
    app: State<'_, Shared>,
    cursor: Option<String>,
) -> CommandResult<ChatsPageResponse> {
    blocking(&app, move |c| list(c, cursor.as_deref())).await
}

/// `create_chat {docIds}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn create_chat(
    app: State<'_, Shared>,
    doc_ids: Option<Vec<String>>,
) -> CommandResult<ChatResponse> {
    blocking(&app, move |c| create(c, doc_ids.as_deref())).await
}

/// `get_chat {chatId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn get_chat(
    app: State<'_, Shared>,
    chat_id: String,
) -> CommandResult<ChatWithMessagesResponse> {
    blocking(&app, move |c| get(c, &chat_id)).await
}

/// `update_chat {chatId, patch}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn update_chat(
    app: State<'_, Shared>,
    chat_id: String,
    patch: serde_json::Value,
) -> CommandResult<ChatResponse> {
    blocking(&app, move |c| update(c, &chat_id, patch)).await
}

/// `delete_chat {chatId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn delete_chat(
    app: State<'_, Shared>,
    chat_id: String,
) -> CommandResult<DeletedResponse> {
    blocking(&app, move |c| delete(c, &chat_id)).await
}

/// `ask {chatId, text, model, onEvent}`. Phase 2 streams text, citation, followups and done
/// events through `on_event`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
// The State plus the four IPC arguments {chatId, text, model, onEvent} are the wire contract.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn ask(
    app: State<'_, Shared>,
    chat_id: String,
    text: String,
    model: Option<String>,
    on_event: Channel<serde_json::Value>,
) -> CommandResult<()> {
    let _unused_until_phase_2 = on_event;
    blocking(&app, move |c| ask_now(c, &chat_id, &text, model.as_deref())).await
}

/// `cancel_ask {chatId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn cancel_ask(app: State<'_, Shared>, chat_id: String) -> CommandResult<()> {
    blocking(&app, move |c| cancel(c, &chat_id)).await
}
