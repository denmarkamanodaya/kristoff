//! `DocStore` on SQLite: `docs` plus `doc_parts`. Listed newest first by id (ids are time-ordered).

use super::{
    SqliteStore, Target, get_u64, insert_failure, sql_int, store_failure, versioned_update,
};
use kristoff_core::domain::cursor::{decode_cursor, doc_position, page_of};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Doc, DocPart, DocStatus, Page, Versioned};
use kristoff_core::ports::{DocPatch, DocStore};
use rusqlite::{Connection, OptionalExtension, Row, params};

const COLUMNS: &str = "id, title, size, status, created_at, page_count, version, file_id";

fn not_found() -> AppError {
    AppError::new(ErrorCode::DocNotFound, "no such doc")
}

fn doc_row(row: &Row<'_>) -> rusqlite::Result<(Doc, i64)> {
    let status: String = row.get(3)?;
    let status = DocStatus::parse(&status).unwrap_or(DocStatus::Failed);
    let doc = Doc {
        id: row.get(0)?,
        title: row.get(1)?,
        size: get_u64(row, 2)?,
        status,
        created_at_ms: row.get(4)?,
        page_count: row.get(5)?,
        parts: vec![],
        file_id: row.get(7)?,
    };
    Ok((doc, row.get(6)?))
}

fn load_parts(conn: &Connection, doc: &mut Doc) -> AppResult<()> {
    let sql = "SELECT n, start_page, end_page, size, file_id FROM doc_parts WHERE doc_id = ?1 \
               ORDER BY n";
    let mut stmt = conn.prepare_cached(sql).map_err(|e| store_failure(&e))?;
    let rows = stmt.query_map([&doc.id], |r| {
        Ok(DocPart {
            n: r.get(0)?,
            start_page: r.get(1)?,
            end_page: r.get(2)?,
            size: get_u64(r, 3)?,
            file_id: r.get(4)?,
        })
    });
    doc.parts = rows
        .and_then(Iterator::collect)
        .map_err(|e| store_failure(&e))?;
    Ok(())
}

fn find(conn: &Connection, doc_id: &str) -> AppResult<Option<Versioned<Doc>>> {
    let sql = format!("SELECT {COLUMNS} FROM docs WHERE id = ?1");
    let found = conn.query_row(&sql, [doc_id], doc_row).optional();
    let Some((mut doc, version)) = found.map_err(|e| store_failure(&e))? else {
        return Ok(None);
    };
    load_parts(conn, &mut doc)?;
    Ok(Some(Versioned {
        value: doc,
        version,
    }))
}

fn write_parts(conn: &Connection, doc: &Doc) -> AppResult<()> {
    conn.execute("DELETE FROM doc_parts WHERE doc_id = ?1", [&doc.id])
        .map_err(|e| store_failure(&e))?;
    let sql = "INSERT INTO doc_parts (doc_id, n, start_page, end_page, size, file_id) \
               VALUES (?1,?2,?3,?4,?5,?6)";
    for p in &doc.parts {
        conn.execute(
            sql,
            params![
                doc.id,
                p.n,
                p.start_page,
                p.end_page,
                sql_int(p.size),
                p.file_id
            ],
        )
        .map_err(|e| store_failure(&e))?;
    }
    Ok(())
}

impl DocStore for SqliteStore {
    fn create_doc(&self, doc: &Doc) -> AppResult<()> {
        self.write(|tx| {
            let sql = format!("INSERT INTO docs ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,1,?7)");
            let values = params![
                doc.id,
                doc.title,
                sql_int(doc.size),
                doc.status.as_str(),
                doc.created_at_ms,
                doc.page_count,
                doc.file_id
            ];
            tx.execute(&sql, values)
                .map_err(|e| insert_failure(&e, "doc"))?;
            write_parts(tx, doc)
        })
    }

    fn get_doc(&self, doc_id: &str) -> AppResult<Versioned<Doc>> {
        self.read(|conn| find(conn, doc_id)?.ok_or_else(not_found))
    }

    fn get_docs(&self, doc_ids: &[String]) -> AppResult<Vec<Doc>> {
        self.read(|conn| {
            let mut out = Vec::new();
            for id in doc_ids {
                out.extend(find(conn, id)?.map(|d| d.value));
            }
            Ok(out)
        })
    }

    fn list_docs(&self, cursor: Option<&str>, limit: usize) -> AppResult<Page<Doc>> {
        let after = cursor.map(decode_cursor).transpose()?.map(|p| p.id);
        self.read(|conn| {
            let sql = format!(
                "SELECT {COLUMNS} FROM docs WHERE (?1 IS NULL OR id < ?1) ORDER BY id DESC LIMIT ?2"
            );
            let mut stmt = conn.prepare_cached(&sql).map_err(|e| store_failure(&e))?;
            let rows = stmt.query_map(params![after, sql_int(limit + 1)], doc_row);
            let docs: Vec<Doc> = rows
                .and_then(|r| r.map(|d| d.map(|(doc, _)| doc)).collect())
                .map_err(|e| store_failure(&e))?;
            let mut page = page_of(docs, limit, doc_position);
            for doc in &mut page.items {
                load_parts(conn, doc)?;
            }
            Ok(page)
        })
    }

    fn update_doc(&self, doc_id: &str, patch: &DocPatch, version: i64) -> AppResult<Doc> {
        self.write(|tx| {
            let mut doc = find(tx, doc_id)?.ok_or_else(not_found)?.value;
            patch.apply_to(&mut doc);
            let sql = "UPDATE docs SET title = ?1, status = ?2, page_count = ?3, file_id = ?4, \
                       version = version + 1 WHERE id = ?5 AND version = ?6";
            let values = params![
                doc.title,
                doc.status.as_str(),
                doc.page_count,
                doc.file_id,
                doc_id,
                version
            ];
            let target = Target {
                table: "docs",
                id: doc_id,
                missing: not_found(),
            };
            versioned_update(tx, sql, values, target)?;
            write_parts(tx, &doc)?;
            Ok(doc)
        })
    }

    fn remove_doc(&self, doc_id: &str) -> AppResult<()> {
        self.delete_row("DELETE FROM docs WHERE id = ?1", doc_id)
            .map(|_| ())
    }
}
