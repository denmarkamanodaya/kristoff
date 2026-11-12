//! `UsageMeter` on SQLite. The daily cap is one conditional upsert (`... WHERE questions < cap`)
//! that runs before the work.

use super::{SqliteStore, get_u64, sql_int, store_failure};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Usage, UsageReport};
use kristoff_core::ports::UsageMeter;
use rusqlite::{Connection, OptionalExtension, params};

fn read_counters(conn: &Connection, sql: &str, key: &dyn rusqlite::ToSql) -> AppResult<Usage> {
    let found = conn
        .query_row(sql, [key], |r| {
            Ok(Usage {
                input: get_u64(r, 0)?,
                output: get_u64(r, 1)?,
                cache_read: get_u64(r, 2)?,
                cache_write: get_u64(r, 3)?,
            })
        })
        .optional();
    Ok(found.map_err(|e| store_failure(&e))?.unwrap_or_default())
}

impl UsageMeter for SqliteStore {
    fn admit_question(&self, day: &str, cap: u32) -> AppResult<()> {
        self.write(|tx| {
            let sql = "INSERT INTO usage_days (day, questions) SELECT ?1, 1 WHERE ?2 > 0 \
                       ON CONFLICT (day) DO UPDATE SET questions = questions + 1 \
                       WHERE questions < ?2";
            let changed = tx
                .execute(sql, params![day, cap])
                .map_err(|e| store_failure(&e))?;
            if changed == 0 {
                return Err(AppError::new(
                    ErrorCode::DailyCapReached,
                    "daily question cap reached",
                ));
            }
            Ok(())
        })
    }

    fn add_tokens(&self, day: &str, usage: &Usage) -> AppResult<()> {
        self.write(|tx| {
            let u = [
                usage.input,
                usage.output,
                usage.cache_read,
                usage.cache_write,
            ]
            .map(sql_int);
            let values = params![day, u[0], u[1], u[2], u[3]];
            let day_sql = "INSERT INTO usage_days (day, input, output, cache_read, cache_write) \
                VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (day) DO UPDATE SET \
                input = input + ?2, output = output + ?3, cache_read = cache_read + ?4, \
                cache_write = cache_write + ?5";
            tx.execute(day_sql, values).map_err(|e| store_failure(&e))?;
            let total_sql = "INSERT INTO usage_total (id, input, output, cache_read, cache_write) \
                VALUES (1, ?1, ?2, ?3, ?4) ON CONFLICT (id) DO UPDATE SET \
                input = input + ?1, output = output + ?2, cache_read = cache_read + ?3, \
                cache_write = cache_write + ?4";
            let totals = params![u[0], u[1], u[2], u[3]];
            tx.execute(total_sql, totals)
                .map_err(|e| store_failure(&e))?;
            Ok(())
        })
    }

    fn read_usage(&self, day: &str) -> AppResult<UsageReport> {
        self.read(|conn| {
            let today = read_counters(
                conn,
                "SELECT input, output, cache_read, cache_write FROM usage_days WHERE day = ?1",
                &day,
            )?;
            let total = read_counters(
                conn,
                "SELECT input, output, cache_read, cache_write FROM usage_total WHERE id = ?1",
                &1,
            )?;
            Ok(UsageReport { today, total })
        })
    }
}
