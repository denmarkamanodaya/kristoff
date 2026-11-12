//! Tauri build script: bakes the Claude API key into the binary (decision D-4) and
//! generates one `allow-<command>` permission per name in `kristoff_contracts::ipc::COMMANDS`.
//!
//! The key comes from the env (`KRISTOFF_ANTHROPIC_KEY`). Debug builds and tests may fall back to
//! the untracked `desktop/.env.local`; release builds never do, and refuse the fake
//! `sk-ant-test...` key. The build fails when it is missing or empty. The value is never printed.
//! There is no TOTP secret any more: the app has no authentication (decision D-17).

// Cargo reads build-script directives (`cargo:rustc-env=...`) from stdout: printing is the
// interface here, not debugging output.
#![allow(clippy::print_stdout)]

use std::path::Path;

const SECRETS: [&str; 1] = ["KRISTOFF_ANTHROPIC_KEY"];

/// `KEY=value` from the dev env file, if the file has it.
fn from_env_file(file: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().trim_matches('"').to_owned())
}

/// Prefix of the fake dev key in `desktop/.env.local`; a release build refuses it.
const FAKE_KEY_PREFIX: &str = "sk-ant-test";

/// One secret's value. Release builds take it from the env only; debug builds and tests may fall
/// back to the dev env file.
fn secret(key: &str, release: bool, env_file: &Path) -> Result<String, String> {
    let from_env = std::env::var(key).ok().filter(|v| !v.is_empty());
    let value = if release {
        from_env
    } else {
        from_env.or_else(|| from_env_file(env_file, key))
    };
    let value = value.filter(|v| !v.is_empty()).ok_or_else(|| {
        let hint = if release {
            "set it in the env (release builds never read .env.local)"
        } else {
            "set it, or add it to desktop/.env.local"
        };
        format!("{key} is missing or empty: {hint}")
    })?;
    if release && value.starts_with(FAKE_KEY_PREFIX) {
        return Err(format!(
            "{key} is the fake dev key; a release build needs the real one"
        ));
    }
    Ok(value)
}

fn bake_secrets() -> Result<(), String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let release = std::env::var("PROFILE").is_ok_and(|p| p == "release");
    let env_file = Path::new(&manifest).join("../../.env.local");
    println!("cargo:rerun-if-changed={}", env_file.display());
    for key in SECRETS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = secret(key, release, &env_file)?;
        println!("cargo:rustc-env={key}={value}");
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bake_secrets()?;
    let manifest = tauri_build::AppManifest::new().commands(kristoff_contracts::ipc::COMMANDS);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))?;
    Ok(())
}
