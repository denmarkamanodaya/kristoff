//! Kristoff desktop shell: the composition root, the TOTP gate and the IPC commands (driving
//! adapters). Plan: docs/desktop/phases/phase-1-core-storage.md §1 and §5.

pub mod commands;
pub mod composition;
pub mod gate;
pub mod scheme;

use composition::Composition;
use std::sync::Arc;
use tauri::Manager;

/// The base32 TOTP secret baked in at build time (`build.rs`; decision D-4).
pub const TOTP_SECRET: &str = env!("KRISTOFF_TOTP_SECRET");

/// The Claude API key baked in at build time; used from Phase 2 (the ask flow).
pub const ANTHROPIC_KEY: &str = env!("KRISTOFF_ANTHROPIC_KEY");

/// Starts the app: opens the app data, manages the composition and registers every command in
/// `kristoff_contracts::ipc::COMMANDS`.
///
/// # Errors
/// Tauri's, when the window or the app data cannot be set up.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let composition = Composition::open(&data_dir, TOTP_SECRET)?;
            app.manage(Arc::new(composition));
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("kristoff", scheme::handle)
        .invoke_handler(tauri::generate_handler![
            commands::session::verify_otp,
            commands::session::get_config,
            commands::session::get_usage,
            commands::docs::list_docs,
            commands::docs::create_doc,
            commands::upload::upload_chunk,
            commands::docs::confirm_doc,
            commands::docs::rename_doc,
            commands::docs::get_doc_link,
            commands::docs::delete_doc,
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::rename_project,
            commands::projects::delete_project,
            commands::chats::list_chats,
            commands::chats::create_chat,
            commands::chats::get_chat,
            commands::chats::update_chat,
            commands::chats::delete_chat,
            commands::chats::ask,
            commands::chats::cancel_ask,
            commands::viewer::open_doc,
            commands::viewer::open_external,
        ])
        .run(tauri::generate_context!())
}
