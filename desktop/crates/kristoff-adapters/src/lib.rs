//! Kristoff driven adapters: the SQLite store (docs, chats, messages, projects, usage, TOTP
//! state), the disk file store, the system clock, random ids and the stderr warning log.
//! Plan: docs/desktop/phases/phase-1-core-storage.md §4; phase-2 §1 (parts, file ids).

pub mod disk_file_store;
pub mod log;
pub mod sqlite;
pub mod system;
