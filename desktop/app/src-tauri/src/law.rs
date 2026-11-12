//! The PH law index the Phase 4 tool loop searches (docs/desktop/phases/phase-3-law-index.md §2).
//! Release builds read `phlaws.db` and the e5 model from the bundle resources, which
//! `tauri.bundle.conf.json` adds only at package time
//! (`cargo tauri build --config app/src-tauri/tauri.bundle.conf.json`), so `cargo tauri build
//! --debug` works without them. Dev builds point at `desktop/data` through the env overrides.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::LawSearch;
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::LawIndex;

/// Env key overriding the database path (dev: `desktop/data/phlaws.db`).
pub const DB_ENV: &str = "KRISTOFF_PHLAWS_DB";
/// Env key overriding the model folder (dev: `desktop/data/model`).
pub const MODEL_ENV: &str = "KRISTOFF_E5_MODEL_DIR";
/// Database path inside the bundle resources.
const BUNDLED_DB: &str = "phlaws.db";
/// Model folder inside the bundle resources.
const BUNDLED_MODEL: &str = "models/e5-small";

/// Where the law database and the e5 model files are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawPaths {
    /// `phlaws.db`.
    pub db: PathBuf,
    /// Folder with `config.json`, `tokenizer.json` and `model.safetensors`.
    pub model_dir: PathBuf,
}

impl LawPaths {
    /// The bundled paths under `resource_dir`, each replaced by its env override when set.
    #[must_use]
    pub fn resolve(resource_dir: &Path, env: impl Fn(&str) -> Option<OsString>) -> Self {
        Self {
            db: env(DB_ENV).map_or_else(|| resource_dir.join(BUNDLED_DB), PathBuf::from),
            model_dir: env(MODEL_ENV)
                .map_or_else(|| resource_dir.join(BUNDLED_MODEL), PathBuf::from),
        }
    }
}

/// Loads the embedder and opens the index read-only (not yet warmed).
///
/// # Errors
/// `INTERNAL` with `LAW_DATA_MISSING` when a file is absent (a build packaged without the law
/// data), or when the model or the database cannot be opened.
pub fn open_law_index(paths: &LawPaths) -> AppResult<Box<dyn LawSearch>> {
    let missing = [paths.db.clone(), paths.model_dir.join("model.safetensors")]
        .into_iter()
        .find(|p| !p.is_file());
    if let Some(path) = missing {
        return Err(AppError::new(
            ErrorCode::Internal,
            format!("LAW_DATA_MISSING: {}", path.display()),
        ));
    }
    let embedder = CandleEmbedder::load(&paths.model_dir, DevicePreference::MetalThenCpu)
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
    Ok(Box::new(LawIndex::open(&paths.db, Box::new(embedder))?))
}
