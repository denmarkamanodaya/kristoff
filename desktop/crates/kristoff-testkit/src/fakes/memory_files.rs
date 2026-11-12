//! `FileStore` in memory: uploads in progress and stored PDFs by doc id.

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::FileStore;
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug, Default)]
struct Files {
    partial: BTreeMap<String, Vec<u8>>,
    stored: BTreeMap<String, Vec<u8>>,
    parts: BTreeMap<(String, u32), Vec<u8>>,
}

/// The in-memory file store.
#[derive(Debug, Default)]
pub struct MemoryFiles(Mutex<Files>);

impl MemoryFiles {
    /// An empty file store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn files(&self) -> MutexGuard<'_, Files> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The committed PDF of `doc_id`, if any (for assertions).
    #[must_use]
    pub fn stored(&self, doc_id: &str) -> Option<Vec<u8>> {
        self.files().stored.get(doc_id).cloned()
    }
}

impl FileStore for MemoryFiles {
    fn partial_len(&self, doc_id: &str) -> AppResult<Option<u64>> {
        Ok(self.files().partial.get(doc_id).map(|b| b.len() as u64))
    }

    fn write_partial(&self, doc_id: &str, offset: u64, bytes: &[u8]) -> AppResult<u64> {
        let mut files = self.files();
        let buf = files.partial.entry(doc_id.to_owned()).or_default();
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let end = start + bytes.len();
        if buf.len() < end {
            buf.resize(end, 0);
        }
        buf[start..end].copy_from_slice(bytes);
        Ok(buf.len() as u64)
    }

    fn partial_head(&self, doc_id: &str, n: usize) -> AppResult<Vec<u8>> {
        let files = self.files();
        let buf = files.partial.get(doc_id).map_or(&[][..], Vec::as_slice);
        Ok(buf.iter().take(n).copied().collect())
    }

    fn write_part(&self, doc_id: &str, n: u32, bytes: &[u8]) -> AppResult<()> {
        self.files()
            .parts
            .insert((doc_id.to_owned(), n), bytes.to_vec());
        Ok(())
    }

    fn commit_partial(&self, doc_id: &str) -> AppResult<()> {
        let mut files = self.files();
        let bytes = files
            .partial
            .remove(doc_id)
            .ok_or_else(|| AppError::new(ErrorCode::Internal, "no upload to commit"))?;
        files.stored.insert(doc_id.to_owned(), bytes);
        Ok(())
    }

    fn read_stored(&self, doc_id: &str, part: Option<u32>) -> AppResult<Vec<u8>> {
        let files = self.files();
        let found = match part {
            None => files.stored.get(doc_id),
            Some(n) => files.parts.get(&(doc_id.to_owned(), n)),
        };
        found
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::DocNotFound, "no such file"))
    }

    fn purge(&self, doc_id: &str) -> AppResult<()> {
        let mut files = self.files();
        files.partial.remove(doc_id);
        files.stored.remove(doc_id);
        files.parts.retain(|(id, _), _| id != doc_id);
        Ok(())
    }
}
