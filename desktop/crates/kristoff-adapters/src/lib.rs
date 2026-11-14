//! Kristoff driven adapters: the SQLite store (docs, chats, messages, projects, usage), the disk file store, the system clock, random ids and the stderr warning log, and the Keychain key store (Phase 8).
//! Plan: docs/desktop/phases/phase-1-core-storage.md §4; phase-2 §1 (parts, file ids).

pub mod disk_file_store;
pub mod keychain;
pub mod log;
pub mod sqlite;
pub mod system;
