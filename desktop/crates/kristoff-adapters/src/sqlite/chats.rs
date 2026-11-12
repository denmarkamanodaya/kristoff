//! `ChatStore` chat half on SQLite: `chats` plus `chat_docs` (attachment ids in order). Listed
//! newest `updated_at` first, tie-break id descending. Messages: `messages.rs`.

use super::{SqliteStore, Target, insert_failure, sql_int, store_failure, versioned_update};
use kristoff_core::domain::cursor::{chat_position, decode_cursor, page_of};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Chat, Page, Versioned};
use kristoff_core::ports::ChatPatch;
use rusqlite::{Connection, OptionalExtension, Row, params};

/// The chat columns, with the attachment ids folded in order.
const SELECT: &str = "SELECT id, title, pinned, project_id, created_at, updated_at, version, \
    (SELECT group_concat(attachment_id, ',' ORDER BY pos) FROM chat_docs WHERE chat_id = id), \
    statutes FROM chats";

pub(super) fn not_found() -> AppError {
    AppError::new(ErrorCode::ChatNotFound, "no such chat")
}

fn chat_row(row: &Row<'_>) -> rusqlite::Result<(Chat, i64)> {
    let doc_ids: Option<String> = row.get(7)?;
    let chat = Chat {
        id: row.get(0)?,
        title: row.get(1)?,
        pinned: row.get(2)?,
        project_id: row.get(3)?,
        created_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
        doc_ids: doc_ids
            .map(|s| s.split(',').map(str::to_owned).collect())
            .unwrap_or_default(),
        statutes: row.get(8)?,
    };
    Ok((chat, row.get(6)?))
}

pub(super) fn find(conn: &Connection, chat_id: &str) -> AppResult<Option<Versioned<Chat>>> {
    let sql = format!("{SELECT} WHERE id = ?1");
    let found = conn.query_row(&sql, [chat_id], chat_row).optional();
    let found = found.map_err(|e| store_failure(&e))?;
    Ok(found.map(|(value, version)| Versioned { value, version }))
}

fn write_doc_ids(conn: &Connection, chat: &Chat) -> AppResult<()> {
    conn.execute("DELETE FROM chat_docs WHERE chat_id = ?1", [&chat.id])
        .map_err(|e| store_failure(&e))?;
    let sql = "INSERT INTO chat_docs (chat_id, pos, attachment_id) VALUES (?1, ?2, ?3)";
    for (pos, id) in chat.doc_ids.iter().enumerate() {
        conn.execute(sql, params![chat.id, sql_int(pos), id])
            .map_err(|e| store_failure(&e))?;
    }
    Ok(())
}

impl SqliteStore {
    pub(super) fn insert_chat(&self, chat: &Chat) -> AppResult<()> {
        self.write(|tx| {
            let sql = "INSERT INTO chats (id, title, pinned, project_id, created_at, updated_at, \
                       version, statutes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)";
            let values = params![
                chat.id,
                chat.title,
                chat.pinned,
                chat.project_id,
                chat.created_at_ms,
                chat.updated_at_ms,
                chat.statutes
            ];
            tx.execute(sql, values)
                .map_err(|e| insert_failure(&e, "chat"))?;
            write_doc_ids(tx, chat)
        })
    }

    pub(super) fn chat_page(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Chat>> {
        let after = cursor.map(decode_cursor).transpose()?;
        let (key, id) = after.map_or((None, None), |p| (Some(p.key), Some(p.id)));
        self.read(|conn| {
            let sql = format!(
                "{SELECT} WHERE (?1 IS NULL OR updated_at < ?1 OR (updated_at = ?1 AND id < ?2)) \
                 ORDER BY updated_at DESC, id DESC LIMIT ?3"
            );
            let mut stmt = conn.prepare_cached(&sql).map_err(|e| store_failure(&e))?;
            let rows = stmt.query_map(params![key, id, sql_int(limit + 1)], chat_row);
            let chats: Vec<Chat> = rows
                .and_then(|r| r.map(|c| c.map(|(chat, _)| chat)).collect())
                .map_err(|e| store_failure(&e))?;
            Ok(page_of(chats, limit, chat_position))
        })
    }

    pub(super) fn patch_chat(
        &self,
        chat_id: &str,
        patch: &ChatPatch,
        version: i64,
    ) -> AppResult<Chat> {
        self.write(|tx| {
            let mut chat = find(tx, chat_id)?.ok_or_else(not_found)?.value;
            patch.apply_to(&mut chat);
            let sql = "UPDATE chats SET title = ?1, pinned = ?2, project_id = ?3, updated_at = ?4, \
                       statutes = ?5, version = version + 1 WHERE id = ?6 AND version = ?7";
            let values = params![
                chat.title,
                chat.pinned,
                chat.project_id,
                chat.updated_at_ms,
                chat.statutes,
                chat_id,
                version
            ];
            let target = Target {
                table: "chats",
                id: chat_id,
                missing: not_found(),
            };
            versioned_update(tx, sql, values, target)?;
            write_doc_ids(tx, &chat)?;
            Ok(chat)
        })
    }

    pub(super) fn remove_chat(&self, chat_id: &str) -> AppResult<()> {
        self.delete_row("DELETE FROM chats WHERE id = ?1", chat_id)
            .map(|_| ())
    }

    pub(super) fn ungroup_chat(&self, chat_id: &str, project_id: &str) -> AppResult<()> {
        self.write(|tx| {
            let sql = "UPDATE chats SET project_id = NULL, version = version + 1 \
                       WHERE id = ?1 AND project_id = ?2";
            tx.execute(sql, [chat_id, project_id])
                .map_err(|e| store_failure(&e))?;
            Ok(())
        })
    }
}
