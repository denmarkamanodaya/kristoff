//! Kristoff desktop entry point.

use std::io::Write;

fn main() {
    if let Err(e) = kristoff_app::run() {
        let _ = writeln!(std::io::stderr(), "Kristoff could not start: {e}");
        std::process::exit(1);
    }
}
