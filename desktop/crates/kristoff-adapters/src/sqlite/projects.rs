//! `ProjectStore` on SQLite. The cap is enforced in the insert itself (`INSERT ... SELECT ...
//! WHERE count < cap` inside `BEGIN IMMEDIATE`), never read-check-write.

use super::{SqliteStore, Target, insert_failure, sql_int, store_failure, versioned_update};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Project, Versioned};
use kristoff_core::ports::{ProjectRename, ProjectStore};
use rusqlite::{Connection, OptionalExtension, Row, params};

const COLUMNS: &str = "id, name, created_at, updated_at, version";

fn not_found() -> AppError {
    AppError::new(ErrorCode::ProjectNotFound, "no such project")
}

fn project_row(row: &Row<'_>) -> rusqlite::Result<Versioned<Project>> {
    let value = Project {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at_ms: row.get(2)?,
        updated_at_ms: row.get(3)?,
    };
    Ok(Versioned {
        value,
        version: row.get(4)?,
    })
}

fn find(conn: &Connection, project_id: &str) -> AppResult<Versioned<Project>> {
    let sql = format!("SELECT {COLUMNS} FROM projects WHERE id = ?1");
    let found = conn.query_row(&sql, [project_id], project_row).optional();
    found.map_err(|e| store_failure(&e))?.ok_or_else(not_found)
}

impl ProjectStore for SqliteStore {
    fn create_project(&self, project: &Project, cap: usize) -> AppResult<()> {
        self.write(|tx| {
            let sql = format!(
                "INSERT INTO projects ({COLUMNS}) SELECT ?1, ?2, ?3, ?4, 1 \
                 WHERE (SELECT count(*) FROM projects) < ?5"
            );
            let p = project;
            let values = params![p.id, p.name, p.created_at_ms, p.updated_at_ms, sql_int(cap)];
            let inserted = tx
                .execute(&sql, values)
                .map_err(|e| insert_failure(&e, "project"))?;
            if inserted == 0 {
                return Err(AppError::new(
                    ErrorCode::ProjectLimitReached,
                    "too many projects",
                ));
            }
            Ok(())
        })
    }

    fn get_project(&self, project_id: &str) -> AppResult<Versioned<Project>> {
        self.read(|conn| find(conn, project_id))
    }

    fn list_projects(&self) -> AppResult<Vec<Project>> {
        self.read(|conn| {
            let sql = format!("SELECT {COLUMNS} FROM projects ORDER BY id");
            let mut stmt = conn.prepare_cached(&sql).map_err(|e| store_failure(&e))?;
            let rows = stmt.query_map([], |r| project_row(r).map(|p| p.value));
            rows.and_then(Iterator::collect)
                .map_err(|e| store_failure(&e))
        })
    }

    fn rename_project(&self, project_id: &str, change: &ProjectRename) -> AppResult<Project> {
        self.write(|tx| {
            let sql = "UPDATE projects SET name = ?1, updated_at = ?2, version = version + 1 \
                       WHERE id = ?3 AND version = ?4";
            let values = params![
                change.name,
                change.updated_at_ms,
                project_id,
                change.version
            ];
            let target = Target {
                table: "projects",
                id: project_id,
                missing: not_found(),
            };
            versioned_update(tx, sql, values, target)?;
            find(tx, project_id).map(|p| p.value)
        })
    }

    fn delete_project(&self, project_id: &str) -> AppResult<()> {
        match self.delete_row("DELETE FROM projects WHERE id = ?1", project_id)? {
            0 => Err(not_found()),
            _ => Ok(()),
        }
    }
}
