//! `StderrLogger`: the `Logger` port as one JSON line per warning on stderr (observability:
//! structured lines, no prompts, answers, titles or keys ever reach it).

use kristoff_core::ports::Logger;
use std::io::Write;

/// Writes `{"level":"warn","event":…,"reason":…}` lines to stderr.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrLogger;

/// The JSON line for one warning (exported for tests).
#[must_use]
pub fn warning_line(event: &str, reason: &str) -> String {
    let line = serde_json::json!({ "level": "warn", "event": event, "reason": reason });
    format!("{line}\n")
}

impl Logger for StderrLogger {
    fn warn(&self, event: &str, reason: &str) {
        // Recovery: a log line that cannot be written is dropped; logging is never error handling.
        if std::io::stderr()
            .write_all(warning_line(event, reason).as_bytes())
            .is_err()
        {}
    }
}

#[cfg(test)]
mod tests {
    use super::warning_line;

    #[test]
    fn a_warning_is_one_json_line() {
        assert_eq!(
            warning_line("files.delete_failed", "UPSTREAM_REJECTED"),
            "{\"event\":\"files.delete_failed\",\"level\":\"warn\",\"reason\":\"UPSTREAM_REJECTED\"}\n"
        );
    }
}
