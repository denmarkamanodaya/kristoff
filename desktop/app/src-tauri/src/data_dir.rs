//! The data-folder carry-over of the rename (Phase 8, docs/desktop/phases/phase-8-lexph-byok.md §2):
//! the bundle id changed from `app.kristoff.desktop` to `ph.lexph.desktop`, and the app data folder
//! follows the id. On the first `LexPH` launch the old folder (threads, PDFs, usage) is COPIED into
//! the new one, so nothing is lost; the old folder is left as it was.

use std::fs;
use std::io;
use std::path::Path;

/// The app data folder name of the Kristoff builds (their bundle id).
pub const OLD_IDENTIFIER: &str = "app.kristoff.desktop";

/// The database whose presence marks a folder as holding app data.
const DB_FILE: &str = "kristoff.db";

/// Copies the Kristoff data folder (a sibling of `new_dir` named [`OLD_IDENTIFIER`]) into `new_dir`.
/// Returns whether anything was copied.
///
/// # Errors
/// The first IO failure. A copy that fails part-way leaves no database in `new_dir` (the database
/// is moved in last), so the next launch tries again.
pub fn carry_over(new_dir: &Path) -> io::Result<bool> {
    match new_dir.parent() {
        Some(parent) => carry_over_from(&parent.join(OLD_IDENTIFIER), new_dir),
        None => Ok(false),
    }
}

/// Copies `old_dir` into `new_dir` when `old_dir` holds app data and `new_dir` does not yet.
/// Entries already in `new_dir` are kept. The database and its WAL files are copied last.
///
/// # Errors
/// The first IO failure.
pub fn carry_over_from(old_dir: &Path, new_dir: &Path) -> io::Result<bool> {
    if !old_dir.join(DB_FILE).is_file() || new_dir.join(DB_FILE).exists() {
        return Ok(false);
    }
    fs::create_dir_all(new_dir)?;
    let mut db_files = Vec::new();
    for entry in fs::read_dir(old_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(DB_FILE) {
            db_files.push(name);
        } else if !new_dir.join(&name).exists() {
            copy_tree(&entry.path(), &new_dir.join(&name))?;
        }
    }
    // The WAL and shared-memory files before the main file, so the database appears complete.
    db_files.sort_by_key(|n| n.len() == DB_FILE.len());
    for name in db_files {
        fs::copy(old_dir.join(&name), new_dir.join(&name))?;
    }
    Ok(true)
}

/// Copies a file, or a folder and everything in it.
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    if !from.is_dir() {
        return fs::copy(from, to).map(|_| ());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        copy_tree(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}
