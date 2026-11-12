//! `UsageMeter` and `AuthStateStore` on SQLite. The daily cap is one conditional upsert
//! (`... WHERE questions < cap`) that runs before the work; replay is one conditional update
//! (`... WHERE last_step < step`).

use super::{SqliteStore, get_u64, sql_int, store_failure};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{TotpState, Usage, UsageReport, Versioned};
use kristoff_core::ports::{AuthStateStore, UsageMeter};
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

impl AuthStateStore for SqliteStore {
    fn get_auth_state(&self) -> AppResult<Versioned<TotpState>> {
        self.read(|conn| {
            let sql =
                "SELECT last_step, fail_count, locked_until, version FROM auth_state WHERE id = 1";
            let found = conn.query_row(sql, [], |r| {
                let value = TotpState {
                    last_step: r.get(0)?,
                    fail_count: r.get(1)?,
                    locked_until_ms: r.get(2)?,
                };
                Ok(Versioned {
                    value,
                    version: r.get(3)?,
                })
            });
            let found = found.optional().map_err(|e| store_failure(&e))?;
            Ok(found.unwrap_or(Versioned {
                value: TotpState::default(),
                version: 0,
            }))
        })
    }

    fn accept_step(&self, step: i64) -> AppResult<()> {
        self.write(|tx| {
            let sql = "INSERT INTO auth_state (id, last_step, fail_count, locked_until, version) \
                       VALUES (1, ?1, 0, 0, 1) ON CONFLICT (id) DO UPDATE SET last_step = ?1, \
                       fail_count = 0, locked_until = 0, version = version + 1 \
                       WHERE last_step < ?1";
            let changed = tx.execute(sql, [step]).map_err(|e| store_failure(&e))?;
            if changed == 0 {
                return Err(AppError::new(ErrorCode::OtpReplayed, "code already used"));
            }
            Ok(())
        })
    }

    fn record_failure(&self, next: &TotpState, version: i64) -> AppResult<()> {
        self.write(|tx| {
            let values = params![
                next.last_step,
                next.fail_count,
                next.locked_until_ms,
                version
            ];
            let sql = if version == 0 {
                "INSERT OR IGNORE INTO auth_state (id, last_step, fail_count, locked_until, \
                 version) SELECT 1, ?1, ?2, ?3, 1 WHERE ?4 = 0"
            } else {
                "UPDATE auth_state SET last_step = ?1, fail_count = ?2, locked_until = ?3, \
                 version = version + 1 WHERE id = 1 AND version = ?4"
            };
            let changed = tx.execute(sql, values).map_err(|e| store_failure(&e))?;
            if changed == 0 {
                let message = "the TOTP state changed since it was read";
                return Err(AppError::new(ErrorCode::VersionConflict, message));
            }
            Ok(())
        })
    }
}
