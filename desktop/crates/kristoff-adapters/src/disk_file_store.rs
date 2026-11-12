//! `DiskFileStore`: PDFs under `<app_data_dir>/docs/`. An upload is written to `<id>.pdf.partial`
//! chunk by chunk at explicit offsets, then renamed to `<id>.pdf` on confirm; parts live in
//! `<id>/part-<n>.pdf`. Delete removes all three.

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::FileStore;
use kristoff_core::util::text::is_hex_id;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The disk file store rooted at a `docs` directory.
#[derive(Debug, Clone)]
pub struct DiskFileStore {
    root: PathBuf,
}

fn io_failure(e: &io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, format!("file storage failed: {e}"))
}

fn ignore_missing(result: io::Result<()>) -> AppResult<()> {
    match result {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_failure(&e)),
        _ => Ok(()),
    }
}

impl DiskFileStore {
    /// A store rooted at `root` (created if missing).
    ///
    /// # Errors
    /// `INTERNAL` when the directory cannot be created.
    pub fn open(root: &Path) -> AppResult<Self> {
        fs::create_dir_all(root).map_err(|e| io_failure(&e))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Refuses anything but a 16-hex id, so no input can name a path outside the root.
    fn checked(doc_id: &str) -> AppResult<&str> {
        if is_hex_id(doc_id) {
            Ok(doc_id)
        } else {
            Err(AppError::new(
                ErrorCode::ValidationFailed,
                "not a document id",
            ))
        }
    }

    fn partial_path(&self, doc_id: &str) -> AppResult<PathBuf> {
        Ok(self
            .root
            .join(format!("{}.pdf.partial", Self::checked(doc_id)?)))
    }

    /// Where the confirmed PDF of `doc_id` lives.
    ///
    /// # Errors
    /// `VALIDATION_FAILED` for a malformed id.
    pub fn pdf_path(&self, doc_id: &str) -> AppResult<PathBuf> {
        Ok(self.root.join(format!("{}.pdf", Self::checked(doc_id)?)))
    }

    /// Where part `n` of `doc_id` lives.
    ///
    /// # Errors
    /// `VALIDATION_FAILED` for a malformed id.
    pub fn part_path(&self, doc_id: &str, n: u32) -> AppResult<PathBuf> {
        Ok(self
            .root
            .join(Self::checked(doc_id)?)
            .join(format!("part-{n}.pdf")))
    }

    /// The confirmed PDF bytes of `doc_id`, if any (tests).
    #[must_use]
    pub fn stored(&self, doc_id: &str) -> Option<Vec<u8>> {
        fs::read(self.pdf_path(doc_id).ok()?).ok()
    }
}

impl FileStore for DiskFileStore {
    fn partial_len(&self, doc_id: &str) -> AppResult<Option<u64>> {
        match fs::metadata(self.partial_path(doc_id)?) {
            Ok(meta) => Ok(Some(meta.len())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_failure(&e)),
        }
    }

    fn write_partial(&self, doc_id: &str, offset: u64, bytes: &[u8]) -> AppResult<u64> {
        let path = self.partial_path(doc_id)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|e| io_failure(&e))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| io_failure(&e))?;
        file.write_all(bytes).map_err(|e| io_failure(&e))?;
        file.metadata().map(|m| m.len()).map_err(|e| io_failure(&e))
    }

    fn partial_head(&self, doc_id: &str, n: usize) -> AppResult<Vec<u8>> {
        let file = fs::File::open(self.partial_path(doc_id)?).map_err(|e| io_failure(&e))?;
        let mut head = Vec::with_capacity(n);
        file.take(n as u64)
            .read_to_end(&mut head)
            .map_err(|e| io_failure(&e))?;
        Ok(head)
    }

    fn write_part(&self, doc_id: &str, n: u32, bytes: &[u8]) -> AppResult<()> {
        let path = self.part_path(doc_id, n)?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| io_failure(&e))?;
        }
        fs::write(path, bytes).map_err(|e| io_failure(&e))
    }

    fn commit_partial(&self, doc_id: &str) -> AppResult<()> {
        fs::rename(self.partial_path(doc_id)?, self.pdf_path(doc_id)?).map_err(|e| io_failure(&e))
    }

    fn read_stored(&self, doc_id: &str, part: Option<u32>) -> AppResult<Vec<u8>> {
        let path = match part {
            None => self.pdf_path(doc_id)?,
            Some(n) => self.part_path(doc_id, n)?,
        };
        fs::read(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => AppError::new(ErrorCode::DocNotFound, "no such file"),
            _ => io_failure(&e),
        })
    }

    fn purge(&self, doc_id: &str) -> AppResult<()> {
        ignore_missing(fs::remove_file(self.pdf_path(doc_id)?))?;
        ignore_missing(fs::remove_file(self.partial_path(doc_id)?))?;
        ignore_missing(fs::remove_dir_all(self.root.join(Self::checked(doc_id)?)))
    }
}
