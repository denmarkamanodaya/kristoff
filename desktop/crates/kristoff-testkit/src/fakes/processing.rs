//! Fakes of the processing ports: a PDF splitter over a tiny fake PDF format, the Files API in
//! memory, and a logger that records warnings.

use kristoff_core::domain::parts::PageRange;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::{Logger, PdfSplitter, RemoteFiles};
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

const FAKE_MAGIC: &str = "%PDF-FAKE ";

/// A fake PDF: a header naming each page's size, then that many filler bytes.
#[must_use]
pub fn fake_pdf(page_sizes: &[usize]) -> Vec<u8> {
    let sizes: Vec<String> = page_sizes.iter().map(ToString::to_string).collect();
    let mut out = format!("{FAKE_MAGIC}{}\n", sizes.join(",")).into_bytes();
    out.resize(out.len() + page_sizes.iter().sum::<usize>(), b'x');
    out
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::UploadInvalid, message)
}

fn page_sizes(pdf: &[u8]) -> AppResult<Vec<usize>> {
    let text = std::str::from_utf8(pdf.split(|b| *b == b'\n').next().unwrap_or_default())
        .map_err(|_| invalid("not a fake PDF"))?;
    let sizes = text
        .strip_prefix(FAKE_MAGIC)
        .ok_or_else(|| invalid("not a fake PDF"))?;
    let sizes: Result<Vec<usize>, _> = sizes.split(',').map(str::parse).collect();
    sizes
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid("no pages"))
}

/// Splits [`fake_pdf`] files.
#[derive(Debug, Default, Clone, Copy)]
pub struct FakeSplitter;

impl PdfSplitter for FakeSplitter {
    fn page_count(&self, pdf: &[u8]) -> AppResult<u32> {
        Ok(u32::try_from(page_sizes(pdf)?.len()).unwrap_or(u32::MAX))
    }

    fn extract(&self, pdf: &[u8], range: PageRange) -> AppResult<Vec<u8>> {
        let sizes = page_sizes(pdf)?;
        let start = usize::try_from(range.start_page).unwrap_or(0);
        let end = usize::try_from(range.end_page).unwrap_or(usize::MAX);
        if start == 0 || end < start || end > sizes.len() {
            return Err(invalid("range outside the PDF"));
        }
        Ok(fake_pdf(&sizes[start - 1..end]))
    }
}

#[derive(Debug, Default)]
struct Remote {
    files: BTreeMap<String, (String, usize)>,
    deleted: Vec<String>,
    uploads: usize,
    fail_from: Option<usize>,
}

/// The Files API in memory: ids `file_mem_<n>`; uploads can be made to fail from the n-th on.
#[derive(Debug, Default)]
pub struct MemoryRemoteFiles(Mutex<Remote>);

impl MemoryRemoteFiles {
    /// An empty remote store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes the `n`-th upload (1-based) and every later one fail with `UPSTREAM_UNAVAILABLE`.
    pub fn fail_from(&self, n: usize) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .fail_from = Some(n);
    }

    /// Ids still stored, with their file name and size.
    #[must_use]
    pub fn stored(&self) -> BTreeMap<String, (String, usize)> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .files
            .clone()
    }

    /// Every id passed to `delete`, in order.
    #[must_use]
    pub fn deleted(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .deleted
            .clone()
    }
}

impl RemoteFiles for MemoryRemoteFiles {
    fn upload(&self, file_name: &str, pdf: &[u8]) -> AppResult<String> {
        let mut remote = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        remote.uploads += 1;
        if remote.fail_from.is_some_and(|n| remote.uploads >= n) {
            return Err(AppError::new(
                ErrorCode::UpstreamUnavailable,
                "upload failed",
            ));
        }
        let id = format!("file_mem_{}", remote.uploads);
        remote
            .files
            .insert(id.clone(), (file_name.to_owned(), pdf.len()));
        Ok(id)
    }

    fn delete(&self, file_ids: &[String]) {
        let mut remote = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        for id in file_ids {
            remote.files.remove(id);
            remote.deleted.push(id.clone());
        }
    }
}

/// Records every warning as `(event, reason)`.
#[derive(Debug, Default)]
pub struct MemoryLogger(Mutex<Vec<(String, String)>>);

impl MemoryLogger {
    /// The warnings so far.
    #[must_use]
    pub fn warnings(&self) -> Vec<(String, String)> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Logger for MemoryLogger {
    fn warn(&self, event: &str, reason: &str) {
        let mut lines = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        lines.push((event.to_owned(), reason.to_owned()));
    }
}
