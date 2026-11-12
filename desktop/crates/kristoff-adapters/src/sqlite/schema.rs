//! The schema and its migrations. `STEPS[n]` takes a file from version `n` to `n + 1`.
//! Every mutable row carries an integer `version`; writes check it (optimistic locking).

use super::store_failure;
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use rusqlite::{Connection, Transaction, TransactionBehavior};

const V1: &str = "
CREATE TABLE docs (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, size INTEGER NOT NULL, status TEXT NOT NULL,
  created_at INTEGER NOT NULL, page_count INTEGER, version INTEGER NOT NULL
) STRICT;
CREATE TABLE doc_parts (
  doc_id TEXT NOT NULL REFERENCES docs(id) ON DELETE CASCADE, n INTEGER NOT NULL,
  start_page INTEGER NOT NULL, end_page INTEGER NOT NULL, size INTEGER NOT NULL,
  PRIMARY KEY (doc_id, n)
) STRICT;
CREATE TABLE projects (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL, version INTEGER NOT NULL
) STRICT;
CREATE TABLE chats (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, pinned INTEGER NOT NULL,
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, version INTEGER NOT NULL
) STRICT;
CREATE INDEX chats_by_updated ON chats(updated_at DESC, id DESC);
CREATE TABLE chat_docs (
  chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE, pos INTEGER NOT NULL,
  attachment_id TEXT NOT NULL, PRIMARY KEY (chat_id, pos)
) STRICT;
CREATE TABLE messages (
  id TEXT PRIMARY KEY, chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
  role TEXT NOT NULL, text TEXT NOT NULL, citations TEXT NOT NULL, usage TEXT, model TEXT,
  stop_reason TEXT, at INTEGER NOT NULL, duration_ms INTEGER, follow_ups TEXT NOT NULL
) STRICT;
CREATE INDEX messages_by_chat ON messages(chat_id, id);
CREATE TABLE usage_days (
  day TEXT PRIMARY KEY, questions INTEGER NOT NULL DEFAULT 0,
  input INTEGER NOT NULL DEFAULT 0, output INTEGER NOT NULL DEFAULT 0,
  cache_read INTEGER NOT NULL DEFAULT 0, cache_write INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE TABLE usage_total (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  input INTEGER NOT NULL DEFAULT 0, output INTEGER NOT NULL DEFAULT 0,
  cache_read INTEGER NOT NULL DEFAULT 0, cache_write INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE TABLE auth_state (
  id INTEGER PRIMARY KEY CHECK (id = 1), last_step INTEGER NOT NULL,
  fail_count INTEGER NOT NULL, locked_until INTEGER NOT NULL, version INTEGER NOT NULL
) STRICT;
";

/// Phase 2: the Files API id of a whole doc and of each part (expand only; nothing is removed).
const V2: &str = "
ALTER TABLE docs ADD COLUMN file_id TEXT;
ALTER TABLE doc_parts ADD COLUMN file_id TEXT;
";

/// Every schema step, oldest first.
const STEPS: [&str; 2] = [V1, V2];

/// The schema version this build writes.
pub(crate) const CURRENT: i64 = 2;

/// Applies every step after the stored version in one immediate transaction.
///
/// # Errors
/// `INTERNAL` when a step fails, or the file is newer than this build.
pub(crate) fn migrate(conn: &Connection) -> AppResult<()> {
    let from: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| store_failure(&e))?;
    if from > CURRENT {
        let message = format!("database schema v{from} is newer than this app's v{CURRENT}");
        return Err(AppError::new(ErrorCode::Internal, message));
    }
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .map_err(|e| store_failure(&e))?;
    for step in STEPS
        .iter()
        .skip(usize::try_from(from).unwrap_or(STEPS.len()))
    {
        tx.execute_batch(step).map_err(|e| store_failure(&e))?;
    }
    tx.pragma_update(None, "user_version", CURRENT)
        .map_err(|e| store_failure(&e))?;
    tx.commit().map_err(|e| store_failure(&e))
}

#[cfg(test)]
mod tests {
    use super::{CURRENT, V1, migrate};
    use rusqlite::Connection;

    #[test]
    fn a_v1_file_migrates_to_v2_and_keeps_its_docs_and_parts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(V1).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute_batch(
            "INSERT INTO docs VALUES ('00000000000000d1','Case',10,'ready',1,150,1);
             INSERT INTO doc_parts VALUES ('00000000000000d1',1,1,100,5);",
        )
        .unwrap();

        migrate(&conn).unwrap();

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        let (title, file_id): (String, Option<String>) = conn
            .query_row("SELECT title, file_id FROM docs", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        let part_file: Option<String> = conn
            .query_row("SELECT file_id FROM doc_parts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            (version, title.as_str(), file_id, part_file),
            (CURRENT, "Case", None, None)
        );
    }
}
