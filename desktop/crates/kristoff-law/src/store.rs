//! Plumbing shared by the two read-only indexes (`LawIndex`, `CaseIndex`): the immutable
//! connection behind a lock, the query embedder, the one-time load of every vector row, and
//! reading one vector row.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

use rusqlite::{Connection, OpenFlags, Row};

use crate::QueryEmbedder;
use crate::error::LawIndexError;
use crate::quant::dot_i8;

/// Vector width (e5-small).
pub(crate) const DIMS: usize = 384;

/// A database opened read-only and immutable, its query embedder, and its vectors once loaded.
pub(crate) struct ReadOnlyDb<V> {
    conn: Mutex<Connection>,
    vectors: OnceLock<V>,
    /// Embeds search queries for this index.
    pub(crate) embedder: Box<dyn QueryEmbedder>,
}

impl<V: Default> ReadOnlyDb<V> {
    /// Opens `path` read-only with `immutable=1` (the bundled file never changes).
    pub(crate) fn open(
        path: &Path,
        embedder: Box<dyn QueryEmbedder>,
    ) -> Result<Self, LawIndexError> {
        let uri = format!("file:{}?immutable=1", path.display());
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(uri, flags)?;
        Ok(Self {
            conn: Mutex::new(conn),
            vectors: OnceLock::new(),
            embedder,
        })
    }

    /// The connection, locked.
    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, LawIndexError> {
        self.conn.lock().map_err(|_| LawIndexError::Poisoned)
    }

    /// The vectors, loaded on first use by running `sql` and `push`ing each row.
    pub(crate) fn vectors(
        &self,
        sql: &str,
        push: impl Fn(&mut V, &Row<'_>) -> Result<(), LawIndexError>,
    ) -> Result<&V, LawIndexError> {
        if let Some(v) = self.vectors.get() {
            return Ok(v);
        }
        let mut loaded = V::default();
        {
            let conn = self.conn()?;
            // A search that arrives during the background warm-up waits on the connection lock;
            // once it gets it the vectors may be loaded, so it must not load them a second time.
            if let Some(v) = self.vectors.get() {
                return Ok(v);
            }
            let mut stmt = conn.prepare(sql)?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                push(&mut loaded, row)?;
            }
        }
        Ok(self.vectors.get_or_init(|| loaded))
    }
}

/// Threads a vector scan splits over.
const SCAN_THREADS: usize = 4;

/// Runs `scan` over `0..len` split into [`SCAN_THREADS`] ranges in parallel and concatenates the
/// results (range order).
pub(crate) fn parallel_scan<T: Send>(
    len: usize,
    scan: impl Fn(std::ops::Range<usize>) -> Vec<T> + Sync,
) -> Vec<T> {
    let per = len.div_ceil(SCAN_THREADS).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..SCAN_THREADS)
            .map(|t| {
                let scan = &scan;
                s.spawn(move || scan(t * per..((t + 1) * per).min(len)))
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}

/// The int8 dot product of query `q8` with stored vector `i` of `data`, times its `scale`.
pub(crate) fn dot_at(data: &[u8], i: usize, q8: &[u8], scale: f32) -> f32 {
    let raw = dot_i8(q8, &data[i * DIMS..(i + 1) * DIMS]);
    // |raw| <= 384 * 127 * 127, well inside f32's exact integer range.
    #[allow(clippy::cast_precision_loss)]
    let scaled = raw as f32 * scale;
    scaled
}

/// Reads `(id, scale, bytes)` from the columns `[id, scale, blob]` of a vector row, rejecting a
/// blob that is not [`DIMS`] bytes.
pub(crate) fn vector_row(
    row: &Row<'_>,
    cols: [usize; 3],
) -> Result<(i64, f32, Vec<u8>), LawIndexError> {
    let id: i64 = row.get(cols[0])?;
    let blob: Vec<u8> = row.get(cols[2])?;
    if blob.len() != DIMS {
        return Err(LawIndexError::BadVector(id));
    }
    let scale: f64 = row.get(cols[1])?;
    // Stored from an f32, so narrowing back is exact.
    #[allow(clippy::cast_possible_truncation)]
    Ok((id, scale as f32, blob))
}
