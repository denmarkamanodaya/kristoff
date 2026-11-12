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

/// Processing runs per doc, bounding the retries at app start (expand only).
const V3: &str = "ALTER TABLE docs ADD COLUMN processing_attempts INTEGER NOT NULL DEFAULT 0;";

/// Phase 4: the per-chat statute search toggle, on unless switched off (expand only).
const V4: &str = "ALTER TABLE chats ADD COLUMN statutes INTEGER NOT NULL DEFAULT 1;";

/// Every schema step, oldest first.
const STEPS: [&str; 4] = [V1, V2, V3, V4];

/// The schema version this build writes.
pub(crate) const CURRENT: i64 = 4;

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
    use super::{CURRENT, STEPS, migrate};
    use rusqlite::Connection;
    use rusqlite::types::FromSql;

    /// The two columns of the single row `sql` selects.
    fn pair<A: FromSql, B: FromSql>(conn: &Connection, sql: &str) -> (A, B) {
        conn.query_row(sql, [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
    }

    /// A file written by an older build: the first `version` steps, then `rows`; then migrated.
    fn migrated_from(version: usize, rows: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        for step in &STEPS[..version] {
            conn.execute_batch(step).unwrap();
        }
        conn.pragma_update(None, "user_version", i64::try_from(version).unwrap())
            .unwrap();
        conn.execute_batch(rows).unwrap();
        migrate(&conn).unwrap();
        let now: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(now, CURRENT);
        conn
    }

    #[test]
    fn a_v1_file_migrates_and_keeps_its_docs_and_parts() {
        let conn = migrated_from(
            1,
            "INSERT INTO docs VALUES ('00000000000000d1','Case',10,'ready',1,150,1);
             INSERT INTO doc_parts VALUES ('00000000000000d1',1,1,100,5);",
        );

        let (title, file_id): (String, Option<String>) =
            pair(&conn, "SELECT title, file_id FROM docs");
        let part_file: Option<String> = conn
            .query_row("SELECT file_id FROM doc_parts", [], |r| r.get(0))
            .unwrap();
        assert_eq!((title.as_str(), file_id, part_file), ("Case", None, None));
    }

    #[test]
    fn a_v2_file_migrates_with_zero_attempts_and_keeps_its_file_ids() {
        let conn = migrated_from(
            2,
            "INSERT INTO docs VALUES ('00000000000000d1','Case',10,'processing',1,NULL,1,'file_x');",
        );

        let (file_id, attempts): (Option<String>, u32) =
            pair(&conn, "SELECT file_id, processing_attempts FROM docs");
        assert_eq!((file_id.as_deref(), attempts), (Some("file_x"), 0));
    }

    #[test]
    fn a_v3_file_migrates_with_statutes_on_for_existing_chats() {
        let conn = migrated_from(
            3,
            "INSERT INTO chats VALUES ('00000000000000c1','Torts',0,NULL,1,2,1);",
        );

        let (title, statutes): (String, bool) = pair(&conn, "SELECT title, statutes FROM chats");
        assert_eq!((title.as_str(), statutes), ("Torts", true));
    }
}
