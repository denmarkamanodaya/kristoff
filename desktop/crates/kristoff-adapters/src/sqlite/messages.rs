//! `ChatStore` on SQLite: the trait impl, with the message half here. Citations, usage and
//! follow-ups are JSON columns written through adapter-private serde rows.

use super::chats::{find, not_found};
use super::{SqliteStore, get_opt_u64, insert_failure, sql_int, store_failure};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Chat, Citation, CitationKind, Message, Page, Role, Usage, Versioned};
use kristoff_core::ports::{ChatPatch, ChatStore};
use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CitationRow {
    n: u32,
    doc_id: Option<String>,
    doc_title: String,
    page: Option<u32>,
    end_page: Option<u32>,
    quote: String,
    kind: String,
    source_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct UsageRow(u64, u64, u64, u64);

// jscpd:ignore-start: the stored row and the wire DTO copy the same fields for different reasons
// to change (the SQLite JSON column versus the IPC contract); coupling them would tie storage to the wire.
fn to_row(c: &Citation) -> CitationRow {
    CitationRow {
        n: c.n,
        doc_id: c.doc_id.clone(),
        doc_title: c.doc_title.clone(),
        page: c.page,
        end_page: c.end_page,
        quote: c.quote.clone(),
        kind: c.kind.as_str().to_owned(),
        source_url: c.source_url.clone(),
    }
}

// jscpd:ignore-end

fn from_row(c: CitationRow) -> Citation {
    Citation {
        n: c.n,
        doc_id: c.doc_id,
        doc_title: c.doc_title,
        page: c.page,
        end_page: c.end_page,
        quote: c.quote,
        kind: CitationKind::parse(&c.kind).unwrap_or(CitationKind::Doc),
        source_url: c.source_url,
    }
}

fn json_failure(e: &serde_json::Error) -> AppError {
    AppError::new(
        ErrorCode::Internal,
        format!("stored message is malformed: {e}"),
    )
}

fn encode(message: &Message) -> AppResult<(String, Option<String>, String)> {
    let citations: Vec<CitationRow> = message.citations.iter().map(to_row).collect();
    let usage = message
        .usage
        .map(|u| UsageRow(u.input, u.output, u.cache_read, u.cache_write));
    let usage = usage
        .map(|u| serde_json::to_string(&u))
        .transpose()
        .map_err(|e| json_failure(&e))?;
    Ok((
        serde_json::to_string(&citations).map_err(|e| json_failure(&e))?,
        usage,
        serde_json::to_string(&message.follow_ups).map_err(|e| json_failure(&e))?,
    ))
}

/// The raw columns of a message row, decoded after the query.
type MessageColumns = (Message, String, Option<String>, String);

fn message_row(row: &Row<'_>) -> rusqlite::Result<MessageColumns> {
    let role: String = row.get(1)?;
    let message = Message {
        id: row.get(0)?,
        role: Role::parse(&role).unwrap_or(Role::Assistant),
        text: row.get(2)?,
        citations: vec![],
        usage: None,
        model: row.get(5)?,
        stop_reason: row.get(6)?,
        at_ms: row.get(7)?,
        duration_ms: get_opt_u64(row, 8)?,
        follow_ups: vec![],
    };
    Ok((message, row.get(3)?, row.get(4)?, row.get(9)?))
}

fn decode((mut message, citations, usage, follow_ups): MessageColumns) -> AppResult<Message> {
    let rows: Vec<CitationRow> = serde_json::from_str(&citations).map_err(|e| json_failure(&e))?;
    message.citations = rows.into_iter().map(from_row).collect();
    let usage: Option<UsageRow> = usage
        .map(|u| serde_json::from_str(&u))
        .transpose()
        .map_err(|e| json_failure(&e))?;
    message.usage = usage.map(|UsageRow(i, o, r, w)| Usage {
        input: i,
        output: o,
        cache_read: r,
        cache_write: w,
    });
    message.follow_ups = serde_json::from_str(&follow_ups).map_err(|e| json_failure(&e))?;
    Ok(message)
}

impl ChatStore for SqliteStore {
    fn create_chat(&self, chat: &Chat) -> AppResult<()> {
        self.insert_chat(chat)
    }

    fn get_chat(&self, chat_id: &str) -> AppResult<Versioned<Chat>> {
        self.read(|conn| find(conn, chat_id)?.ok_or_else(not_found))
    }

    fn list_chats(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Chat>> {
        self.chat_page(cursor, limit)
    }

    fn update_chat(&self, chat_id: &str, patch: &ChatPatch, version: i64) -> AppResult<Chat> {
        self.patch_chat(chat_id, patch, version)
    }

    fn put_message(&self, chat_id: &str, message: &Message) -> AppResult<()> {
        let (citations, usage, follow_ups) = encode(message)?;
        self.write(|tx| {
            find(tx, chat_id)?.ok_or_else(not_found)?;
            let sql = "INSERT INTO messages (id, chat_id, role, text, citations, usage, model, \
                       stop_reason, at, duration_ms, follow_ups) \
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";
            let m = message;
            let values = params![
                m.id,
                chat_id,
                m.role.as_str(),
                m.text,
                citations,
                usage,
                m.model,
                m.stop_reason,
                m.at_ms,
                m.duration_ms.map(sql_int),
                follow_ups
            ];
            tx.execute(sql, values)
                .map_err(|e| insert_failure(&e, "message"))?;
            Ok(())
        })
    }

    fn recent_messages(&self, chat_id: &str, limit: usize) -> AppResult<Vec<Message>> {
        let rows: Vec<MessageColumns> = self.read(|conn| {
            let sql = "SELECT id, role, text, citations, usage, model, stop_reason, at, \
                       duration_ms, follow_ups FROM messages WHERE chat_id = ?1 \
                       ORDER BY id DESC LIMIT ?2";
            let mut stmt = conn.prepare_cached(sql).map_err(|e| store_failure(&e))?;
            let rows = stmt.query_map(params![chat_id, sql_int(limit)], message_row);
            rows.and_then(Iterator::collect)
                .map_err(|e| store_failure(&e))
        })?;
        rows.into_iter().rev().map(decode).collect()
    }

    fn delete_messages(&self, chat_id: &str) -> AppResult<usize> {
        self.delete_row("DELETE FROM messages WHERE chat_id = ?1", chat_id)
    }

    fn delete_chat(&self, chat_id: &str) -> AppResult<()> {
        self.remove_chat(chat_id)
    }

    fn clear_project(&self, chat_id: &str, project_id: &str) -> AppResult<()> {
        self.ungroup_chat(chat_id, project_id)
    }
}
