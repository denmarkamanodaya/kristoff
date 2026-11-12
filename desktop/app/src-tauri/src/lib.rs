//! Kristoff desktop shell: the composition root, the TOTP gate and the IPC commands (driving
//! adapters). Plan: docs/desktop/phases/phase-1-core-storage.md §1 and §5.

pub mod ai;
pub mod asks;
pub mod commands;
pub mod composition;
pub mod gate;
pub mod scheme;

use composition::Composition;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};

/// The label of the main window (tauri.conf.json); closing it cancels every running ask.
const MAIN_WINDOW: &str = "main";

/// The base32 TOTP secret baked in at build time (`build.rs`; decision D-4).
pub const TOTP_SECRET: &str = env!("KRISTOFF_TOTP_SECRET");

/// The Claude API key baked in at build time; used from Phase 2 (the ask flow).
pub const ANTHROPIC_KEY: &str = env!("KRISTOFF_ANTHROPIC_KEY");

/// Opens the app data, manages the composition and resumes docs left mid-processing.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let composition = Arc::new(Composition::open(&data_dir, TOTP_SECRET, ANTHROPIC_KEY)?);
    let resume = composition.docs_to_resume()?;
    app.manage(Arc::clone(&composition));
    commands::docs::spawn_processing(composition, resume);
    Ok(())
}

/// Closing the main window cancels every running ask (their partial answers are stored).
fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let closing = matches!(
        event,
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
    );
    if closing && window.label() == MAIN_WINDOW {
        window.state::<commands::Shared>().asks.cancel_all();
    }
}

/// Starts the app: opens the app data, manages the composition and registers every command in
/// `kristoff_contracts::ipc::COMMANDS`.
///
/// # Errors
/// Tauri's, when the window or the app data cannot be set up.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .setup(setup)
        .on_window_event(on_window_event)
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
            commands::ask::ask,
            commands::ask::cancel_ask,
            commands::viewer::open_doc,
            commands::viewer::open_external,
        ])
        .run(tauri::generate_context!())
}
