//! Small database and lock-file helpers shared by every stage.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use kristoff_core::util::time::utc_day;
use rusqlite::{Connection, OptionalExtension, params};

/// Reads `# revision: <sha>` from a lock file.
pub fn lock_revision(path: &Path) -> Result<String> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    text.lines()
        .find_map(|l| l.strip_prefix("# revision: "))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{}: no revision line", path.display()))
}

/// Writes one `meta` row.
pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

/// Reads one `meta` row.
pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

/// True once a stage marked itself done.
pub fn stage_done(conn: &Connection, key: &str) -> Result<bool> {
    Ok(get_meta(conn, key)?.as_deref() == Some("done"))
}

/// Opens (creating with `create_sql` when new) a database in WAL mode for the build.
pub fn open(path: &Path, create_sql: &str) -> Result<Connection> {
    let fresh = !path.exists();
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // A second embedding run may write the same db; wait for its batch instead of failing.
    conn.busy_timeout(std::time::Duration::from_secs(120))?;
    if fresh {
        conn.execute_batch(create_sql)?;
    }
    Ok(conn)
}

/// Rows in `table` (a fixed table name from this crate, never input).
pub fn count(conn: &Connection, table: &str) -> Result<i64> {
    Ok(conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?)
}

/// Switches back to a rollback journal and compacts the file; returns its size in bytes.
pub fn compact(conn: &Connection, path: &Path) -> Result<u64> {
    conn.pragma_update(None, "journal_mode", "DELETE")?;
    conn.execute_batch("VACUUM;")?;
    Ok(std::fs::metadata(path)?.len())
}

/// Today's UTC date `yyyy-mm-dd` from the system clock (the bin is the composition root).
pub fn build_date() -> Result<String> {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    Ok(utc_day(i64::try_from(ms)?))
}
