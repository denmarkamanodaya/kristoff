//! `SettingsStore` on SQLite (Phase 8): one `settings` row; each write bumps its `version`.

use super::{SqliteStore, store_failure};
use kristoff_core::error::AppResult;
use kristoff_core::ports::SettingsStore;
use rusqlite::OptionalExtension;

impl SettingsStore for SqliteStore {
    fn daily_question_cap(&self) -> AppResult<Option<u32>> {
        self.read(|conn| {
            let found: Option<Option<u32>> = conn
                .query_row(
                    "SELECT daily_question_cap FROM settings WHERE id = 1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| store_failure(&e))?;
            Ok(found.flatten())
        })
    }

    fn set_daily_question_cap(&self, cap: u32) -> AppResult<()> {
        self.write(|tx| {
            let sql = "INSERT INTO settings (id, daily_question_cap, version) VALUES (1, ?1, 1) \
                       ON CONFLICT (id) DO UPDATE SET daily_question_cap = ?1, \
                       version = version + 1";
            tx.execute(sql, [cap]).map_err(|e| store_failure(&e))?;
            Ok(())
        })
    }
}
