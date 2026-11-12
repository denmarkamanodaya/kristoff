//! `ChatStore` on `MemoryStore`.

use super::memory_store::{MemoryStore, conflict, duplicate, page_desc};
use kristoff_core::domain::cursor::chat_position;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Chat, Message, Page, Versioned};
use kristoff_core::ports::{ChatPatch, ChatStore};

fn not_found() -> AppError {
    AppError::new(ErrorCode::ChatNotFound, "no such chat")
}

impl ChatStore for MemoryStore {
    fn create_chat(&self, chat: &Chat) -> AppResult<()> {
        let mut state = self.state();
        if state.chats.contains_key(&chat.id) {
            return Err(duplicate("chat"));
        }
        state.chats.insert(
            chat.id.clone(),
            Versioned {
                value: chat.clone(),
                version: 1,
            },
        );
        Ok(())
    }

    fn get_chat(&self, chat_id: &str) -> AppResult<Versioned<Chat>> {
        self.state()
            .chats
            .get(chat_id)
            .cloned()
            .ok_or_else(not_found)
    }

    fn list_chats(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Chat>> {
        let mut all: Vec<Chat> = self
            .state()
            .chats
            .values()
            .map(|c| c.value.clone())
            .collect();
        all.sort_by(|a, b| (b.updated_at_ms, &b.id).cmp(&(a.updated_at_ms, &a.id)));
        page_desc(&all, cursor, limit, chat_position)
    }

    fn update_chat(&self, chat_id: &str, patch: &ChatPatch, version: i64) -> AppResult<Chat> {
        let mut state = self.state();
        let stored = state.chats.get_mut(chat_id).ok_or_else(not_found)?;
        if stored.version != version {
            return Err(conflict());
        }
        patch.apply_to(&mut stored.value);
        stored.version += 1;
        Ok(stored.value.clone())
    }

    fn put_message(&self, chat_id: &str, message: &Message) -> AppResult<()> {
        let mut state = self.state();
        if !state.chats.contains_key(chat_id) {
            return Err(not_found());
        }
        if state.messages.iter().any(|(_, m)| m.id == message.id) {
            return Err(duplicate("message"));
        }
        state.messages.push((chat_id.to_owned(), message.clone()));
        Ok(())
    }

    fn recent_messages(&self, chat_id: &str, limit: usize) -> AppResult<Vec<Message>> {
        let state = self.state();
        let mut mine: Vec<Message> = state
            .messages
            .iter()
            .filter(|(c, _)| c == chat_id)
            .map(|(_, m)| m.clone())
            .collect();
        mine.sort_by(|a, b| a.id.cmp(&b.id));
        let skip = mine.len().saturating_sub(limit);
        Ok(mine.into_iter().skip(skip).collect())
    }

    fn delete_messages(&self, chat_id: &str) -> AppResult<usize> {
        let mut state = self.state();
        let before = state.messages.len();
        state.messages.retain(|(c, _)| c != chat_id);
        Ok(before - state.messages.len())
    }

    fn delete_chat(&self, chat_id: &str) -> AppResult<()> {
        self.state().chats.remove(chat_id);
        Ok(())
    }

    fn clear_project(&self, chat_id: &str, project_id: &str) -> AppResult<()> {
        let mut state = self.state();
        if let Some(stored) = state.chats.get_mut(chat_id)
            && stored.value.project_id.as_deref() == Some(project_id)
        {
            stored.value.project_id = None;
            stored.version += 1;
        }
        Ok(())
    }
}
