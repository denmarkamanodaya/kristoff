//! `SqliteStore`: one SQLite file (`<app_data_dir>/kristoff.db`) behind the doc, chat, project,
//! usage and settings ports. WAL, foreign keys on, a 2 s busy wait, every write in
//! `BEGIN IMMEDIATE`, migrations through `PRAGMA user_version` (pattern:
//! `dexfs/crates/footage-adapters/src/sqlite_setup.rs`).

mod chats;
mod docs;
mod messages;
mod projects;
mod schema;
mod settings;
mod usage;

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// How long a connection waits for another connection's write lock.
const BUSY_WAIT: Duration = Duration::from_secs(2);

/// The SQLite store.
#[derive(Debug)]
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

/// Maps a SQLite failure to `INTERNAL` (the message stays in logs only, never on the wire).
pub(crate) fn store_failure(e: &rusqlite::Error) -> AppError {
    AppError::new(ErrorCode::Internal, format!("storage failed: {e}"))
}

impl SqliteStore {
    /// Opens (creating if needed) the database at `path` and migrates it.
    ///
    /// # Errors
    /// `INTERNAL` when the file cannot be opened or migrated, or was written by a newer app.
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path).map_err(|e| store_failure(&e))?;
        Self::prepare(conn)
    }

    /// An in-memory database (tests).
    ///
    /// # Errors
    /// `INTERNAL` when it cannot be created.
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| store_failure(&e))?;
        Self::prepare(conn)
    }

    fn prepare(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| store_failure(&e))?;
        conn.busy_timeout(BUSY_WAIT)
            .map_err(|e| store_failure(&e))?;
        let _mode: String = conn
            .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
            .map_err(|e| store_failure(&e))?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `read` on the connection.
    pub(crate) fn read<T>(&self, read: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        read(&self.lock())
    }

    /// Runs `write` in one `BEGIN IMMEDIATE` transaction; commits only when it succeeds.
    pub(crate) fn write<T>(
        &self,
        write: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let conn = self.lock();
        let tx = Transaction::new_unchecked(&conn, TransactionBehavior::Immediate)
            .map_err(|e| store_failure(&e))?;
        let value = write(&tx)?;
        tx.commit().map_err(|e| store_failure(&e))?;
        Ok(value)
    }
}

impl SqliteStore {
    /// Deletes the row `id` with `sql` (`DELETE ... WHERE id = ?1`) in its own transaction;
    /// returns how many rows went.
    pub(crate) fn delete_row(&self, sql: &str, id: &str) -> AppResult<usize> {
        self.write(|tx| tx.execute(sql, [id]).map_err(|e| store_failure(&e)))
    }
}

/// The row a versioned update targets, and the error its absence gets.
pub(crate) struct Target<'a> {
    /// Table name (a constant, never input).
    pub table: &'static str,
    /// Row id.
    pub id: &'a str,
    /// The not-found error of that table.
    pub missing: AppError,
}

/// Runs an optimistic-lock `UPDATE ... WHERE id = ? AND version = ?`. When it matched no row,
/// fails with `target.missing` if the row is absent, else `VERSION_CONFLICT`.
pub(crate) fn versioned_update(
    tx: &Transaction<'_>,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
    target: Target<'_>,
) -> AppResult<()> {
    if tx.execute(sql, values).map_err(|e| store_failure(&e))? > 0 {
        return Ok(());
    }
    let exists = format!("SELECT 1 FROM {} WHERE id = ?1", target.table);
    match tx.query_row(&exists, [target.id], |_| Ok(())) {
        Ok(()) => Err(AppError::new(
            ErrorCode::VersionConflict,
            "the item changed since it was read",
        )),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(target.missing),
        Err(e) => Err(store_failure(&e)),
    }
}

/// An unsigned count as a SQLite integer (rusqlite binds `i64`; counts never reach `i64::MAX`).
pub(crate) fn sql_int(n: impl TryInto<i64>) -> i64 {
    n.try_into().unwrap_or(i64::MAX)
}

/// Reads column `i` as a non-negative integer.
pub(crate) fn get_u64(row: &rusqlite::Row<'_>, i: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(i)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(i, value))
}

/// Reads column `i` as an optional non-negative integer.
pub(crate) fn get_opt_u64(row: &rusqlite::Row<'_>, i: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(i)?;
    value
        .map(|v| u64::try_from(v).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(i, v)))
        .transpose()
}

/// Maps an insert failure: a duplicate key becomes `INTERNAL` "already exists".
pub(crate) fn insert_failure(e: &rusqlite::Error, what: &str) -> AppError {
    match e.sqlite_error_code() {
        Some(rusqlite::ErrorCode::ConstraintViolation) => {
            AppError::new(ErrorCode::Internal, format!("{what} already exists"))
        }
        _ => store_failure(e),
    }
}
