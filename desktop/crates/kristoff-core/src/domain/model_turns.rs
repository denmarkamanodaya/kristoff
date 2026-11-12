//! Builds the model turns for one question (docs/contract.md "Model call"): the last N stored
//! messages from the first user message on, same-role neighbours merged, `"(no answer)"` for empty
//! text, then the new question; the attached docs go only in the first user turn, with the cache
//! breakpoint on the last doc. Ported from `backend/src/domain/chats/build-model-turns/v1.ts`.

use crate::domain::model_request::{AttachedDoc, ModelTurn, TurnBlock};
use crate::model::{Message, Role};

/// Text sent for a stored message whose text is empty (as the original server did).
pub const EMPTY_TEXT: &str = "(no answer)";

/// Joins consecutive turns of the same role, so roles always alternate.
fn alternate(turns: impl Iterator<Item = (Role, String)>) -> Vec<(Role, String)> {
    let mut out: Vec<(Role, String)> = Vec::new();
    for (role, text) in turns {
        match out.last_mut() {
            Some((last_role, last_text)) if *last_role == role => {
                last_text.push_str("\n\n");
                last_text.push_str(&text);
            }
            _ => out.push((role, text)),
        }
    }
    out
}

fn doc_blocks(docs: &[AttachedDoc]) -> impl Iterator<Item = TurnBlock> + '_ {
    docs.iter().enumerate().map(|(i, doc)| TurnBlock::Doc {
        file_id: doc.file_id.clone(),
        title: doc.title.clone(),
        is_cache_breakpoint: i + 1 == docs.len(),
    })
}

/// The turns for one question.
///
/// `history` is the chat's stored messages, oldest first, before the question; `docs` are the
/// usable attachments in the order the model indexes them; at most `max_history_turns` past
/// messages are sent. The result alternates roles and starts and ends with a user turn.
#[must_use]
pub fn build_model_turns(
    history: &[Message],
    question: &str,
    docs: &[AttachedDoc],
    max_history_turns: usize,
) -> Vec<ModelTurn> {
    let window = &history[history.len().saturating_sub(max_history_turns)..];
    let past = window.iter().skip_while(|m| m.role != Role::User).map(|m| {
        let text = if m.text.is_empty() {
            EMPTY_TEXT
        } else {
            &m.text
        };
        (m.role, text.to_owned())
    });
    let turns = alternate(past.chain(std::iter::once((Role::User, question.to_owned()))));
    turns
        .into_iter()
        .enumerate()
        .map(|(i, (role, text))| {
            let docs: &[AttachedDoc] = if i == 0 { docs } else { &[] };
            let blocks = doc_blocks(docs).chain(std::iter::once(TurnBlock::Text(text)));
            ModelTurn {
                role,
                blocks: blocks.collect(),
            }
        })
        .collect()
}
