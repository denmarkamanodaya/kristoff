//! Tauri build script: generates one `allow-<command>` permission per name in
//! `kristoff_contracts::ipc::COMMANDS`. Nothing secret is baked in any more: each user saves their
//! own Claude key in Settings (Phase 8; it replaced decision D-4's baked key).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = tauri_build::AppManifest::new().commands(kristoff_contracts::ipc::COMMANDS);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))?;
    Ok(())
}
