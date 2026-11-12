//! `DocStore` on `MemoryStore`.

use super::memory_store::{MemoryStore, conflict, duplicate, page_desc};
use kristoff_core::domain::cursor::doc_position;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Doc, Page, Versioned};
use kristoff_core::ports::{DocPatch, DocStore};

fn not_found() -> AppError {
    AppError::new(ErrorCode::DocNotFound, "no such doc")
}

impl DocStore for MemoryStore {
    fn create_doc(&self, doc: &Doc) -> AppResult<()> {
        let mut state = self.state();
        if state.docs.contains_key(&doc.id) {
            return Err(duplicate("doc"));
        }
        state.docs.insert(
            doc.id.clone(),
            Versioned {
                value: doc.clone(),
                version: 1,
            },
        );
        Ok(())
    }

    fn get_doc(&self, doc_id: &str) -> AppResult<Versioned<Doc>> {
        self.state().docs.get(doc_id).cloned().ok_or_else(not_found)
    }

    fn get_docs(&self, doc_ids: &[String]) -> AppResult<Vec<Doc>> {
        let state = self.state();
        Ok(doc_ids
            .iter()
            .filter_map(|id| state.docs.get(id))
            .map(|d| d.value.clone())
            .collect())
    }

    fn list_docs(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Doc>> {
        let all: Vec<Doc> = self
            .state()
            .docs
            .values()
            .rev()
            .map(|d| d.value.clone())
            .collect();
        page_desc(&all, cursor, limit, doc_position)
    }

    fn update_doc(&self, doc_id: &str, patch: &DocPatch, version: i64) -> AppResult<Doc> {
        let mut state = self.state();
        let stored = state.docs.get_mut(doc_id).ok_or_else(not_found)?;
        if stored.version != version {
            return Err(conflict());
        }
        patch.apply_to(&mut stored.value);
        stored.version += 1;
        Ok(stored.value.clone())
    }

    fn remove_doc(&self, doc_id: &str) -> AppResult<()> {
        self.state().docs.remove(doc_id);
        Ok(())
    }
}
