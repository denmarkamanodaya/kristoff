//! The `LexPH` (was Kristoff) desktop shell: the composition root and the IPC commands (driving
//! adapters). No authentication (decision D-17); each user brings their own Claude key (Phase 8).
//! Plan: docs/desktop/phases/phase-1-core-storage.md §1 and §5; phase-8-lexph-byok.md.

pub mod ai;
pub mod asks;
pub mod commands;
pub mod composition;
pub mod data_dir;
pub mod law;
pub mod scheme;

use composition::Composition;
use kristoff_contracts::ipc::LAW_READY_EVENT;
use kristoff_core::ports::Logger;
use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};

/// The label of the main window (tauri.conf.json); closing it cancels every running ask.
const MAIN_WINDOW: &str = "main";

/// Opens the app data, manages the composition and resumes docs left mid-processing.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    if let Err(e) = data_dir::carry_over(&data_dir) {
        // Recovery: the app still opens on a fresh folder; the old data stays where it was.
        let reason = format!("{:?}", e.kind());
        kristoff_adapters::log::StderrLogger.warn("data.carry_over_failed", &reason);
    }
    let composition = Arc::new(Composition::open(&data_dir)?);
    let resume = composition.docs_to_resume()?;
    app.manage(Arc::clone(&composition));
    let law_paths = law::LawPaths::resolve(&app.path().resource_dir()?, |k| std::env::var_os(k));
    open_law(&composition, &law_paths, app.handle().clone());
    commands::docs::spawn_processing(composition, resume);
    Ok(())
}

/// Opens the law index on the launch path (an immutable SQLite open, so `get_config` reports
/// statutes from the first call) and warms its vectors and model off the UI thread. A missing
/// index or a failed warm-up is logged; the app then runs without law search, or its searches fail.
/// When the warm-up ends (or there is nothing to warm) it emits `law-ready`.
fn open_law(composition: &Arc<Composition>, paths: &law::LawPaths, handle: tauri::AppHandle) {
    if let Err(e) = law::open_and_install_all(composition, paths) {
        composition.warn("law.unavailable", e.code.as_str());
    }
    let warming = Arc::clone(composition);
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = warming.warm_law() {
            warming.warn("law.warm_failed", e.code.as_str());
        }
        if handle.emit(LAW_READY_EVENT, ()).is_err() {
            warming.warn("law.ready_event", "EMIT_FAILED");
        }
    });
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
            commands::session::get_config,
            commands::keys::get_key_status,
            commands::keys::set_api_key,
            commands::keys::remove_api_key,
            commands::keys::get_settings,
            commands::keys::update_settings,
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
