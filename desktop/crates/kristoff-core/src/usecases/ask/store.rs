//! Stores the question and the answer of one ask.

use super::{Answer, AskDeps, AskOutcome, final_text, message};
use crate::domain::model_request::FollowUps;
use crate::error::AppResult;
use crate::limits::{AUTO_TITLE_CHARS, NEW_CHAT_TITLE};
use crate::model::{Chat, Role, Usage, Versioned};
use crate::ports::{ChatChanges, ChatPatch};
use crate::util::text::truncate_utf16;
use crate::util::time::utc_day;

/// Appended to an answer that stopped at `max_tokens`.
pub const CUT_OFF_NOTE: &str =
    "\n\n_(Answer cut off at the length limit. Ask \"continue\" for the rest.)_";
/// Appended to an answer whose call failed or was cancelled.
pub const FAILED_NOTE: &str = "\n\n_(The answer was interrupted. Please ask again.)_";

/// The chat title from a first question: its first 60 characters (contract Clarifications).
fn auto_title(text: &str) -> String {
    let title = truncate_utf16(text.trim(), AUTO_TITLE_CHARS);
    if title.is_empty() {
        NEW_CHAT_TITLE
    } else {
        title
    }
    .to_owned()
}

/// Stores the user message, then bumps the chat (and titles it on its first question).
pub(super) fn store_question(
    deps: &AskDeps<'_>,
    chat: &Versioned<Chat>,
    text: &str,
    is_first: bool,
) -> AppResult<()> {
    let at_ms = deps.clock.now_ms();
    let question = message(deps.ids.new_ulid(), Role::User, text.to_owned(), at_ms);
    deps.chats.put_message(&chat.value.id, &question)?;
    let changes = ChatChanges {
        title: is_first.then(|| auto_title(text)),
        ..ChatChanges::default()
    };
    let patch = ChatPatch {
        changes,
        updated_at_ms: at_ms,
    };
    deps.chats
        .update_chat(&chat.value.id, &patch, chat.version)
        .map(|_| ())
}

fn add(a: Usage, b: Option<Usage>) -> Usage {
    let b = b.unwrap_or_default();
    Usage {
        input: a.input + b.input,
        output: a.output + b.output,
        cache_read: a.cache_read + b.cache_read,
        cache_write: a.cache_write + b.cache_write,
    }
}

/// ALWAYS stores the assistant message (a failed answer keeps its text plus `FAILED_NOTE` and
/// `stopReason: "error"`), then adds the answer's and the follow-ups' tokens to today.
pub(super) fn store_answer(
    deps: &AskDeps<'_>,
    chat_id: &str,
    (answer, follow_ups): (Answer, FollowUps),
    started_ms: i64,
) -> AppResult<AskOutcome> {
    let now = deps.clock.now_ms();
    let note = match &answer.end {
        Err(_) => FAILED_NOTE,
        Ok(end) if end.stop_reason.as_deref() == Some("max_tokens") => CUT_OFF_NOTE,
        Ok(_) => "",
    };
    let mut stored = message(
        deps.ids.new_ulid(),
        Role::Assistant,
        final_text(&answer.text, note),
        now,
    );
    stored.citations = answer.citations;
    stored.model = Some(answer.model);
    stored.duration_ms = Some(u64::try_from(now - started_ms).unwrap_or(0));
    stored.follow_ups = follow_ups.questions;
    let end = match answer.end {
        Ok(end) => end,
        Err(e) => {
            stored.stop_reason = Some("error".to_owned());
            deps.chats.put_message(chat_id, &stored)?;
            return Err(e);
        }
    };
    stored.usage = Some(end.usage);
    stored.stop_reason = end.stop_reason;
    deps.chats.put_message(chat_id, &stored)?;
    deps.meter
        .add_tokens(&utc_day(now), &add(end.usage, follow_ups.usage))?;
    Ok(AskOutcome {
        message: stored,
        usage: end.usage,
    })
}
