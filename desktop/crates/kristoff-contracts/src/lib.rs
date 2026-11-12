//! Kristoff wire contract on the desktop: serde shapes equal to `backend/src/contracts/shapes.ts`
//! (and `docs/contract.md`), the error envelope, request checks with the `limits.ts` bounds, the
//! IPC command list and the app configuration. `contracts/fixtures/*.json` pin every shape.
//! Plan: docs/desktop/phases/phase-1-core-storage.md §2.

pub mod ask_event;
pub mod config;
pub mod envelopes;
pub mod error;
pub mod ipc;
pub mod model_config;
pub mod requests;
pub mod shapes;
pub mod wire;
