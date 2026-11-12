//! Kristoff driven adapters: the SQLite store (docs, chats, messages, projects, usage, TOTP
//! state), the disk file store, the system clock, random ids, and the Phase-1 doc processor.
//! Plan: docs/desktop/phases/phase-1-core-storage.md §4.

pub mod disk_file_store;
pub mod pass_through_processor;
pub mod sqlite;
pub mod system;
