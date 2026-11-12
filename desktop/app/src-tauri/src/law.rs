//! The PH law index the Phase 4 tool loop searches (docs/desktop/phases/phase-3-law-index.md §2).
//! Release builds read `phlaws.db` and the e5 model from the bundle resources, which
//! `tauri.bundle.conf.json` adds only at package time
//! (`cargo tauri build --config app/src-tauri/tauri.bundle.conf.json`), so `cargo tauri build
//! --debug` works without them. Dev builds point at `desktop/data` through the env overrides.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::ports::LawSearch;
use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::{LawIndex, QueryEmbedder};

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

/// The e5 embedder, loaded on first use: the 235 MB model load stays off the launch path. A
/// caller that arrives while another thread loads it blocks until the load ends (`OnceLock`), so
/// a search during warm-up waits instead of failing.
struct LazyEmbedder {
    dir: PathBuf,
    loaded: OnceLock<Result<CandleEmbedder, String>>,
}

impl LazyEmbedder {
    fn get(&self) -> Result<&CandleEmbedder, String> {
        let loaded = self.loaded.get_or_init(|| {
            CandleEmbedder::load(&self.dir, DevicePreference::MetalThenCpu)
                .map_err(|e| e.to_string())
        });
        loaded.as_ref().map_err(Clone::clone)
    }
}

impl QueryEmbedder for LazyEmbedder {
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        self.get()?.embed_query(text)
    }

    fn warm(&self) -> Result<(), String> {
        QueryEmbedder::warm(self.get()?)
    }
}

/// Opens the index read-only with a lazily loaded embedder. Cheap (an immutable SQLite open),
/// so setup runs it on the launch path and `get_config` sees the index at once; call
/// [`LawSearch::warm`] afterwards, off the UI thread, to load the vectors and the model.
///
/// # Errors
/// `INTERNAL` with `LAW_DATA_MISSING` when a file is absent (a build packaged without the law
/// data), or when the database cannot be opened.
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
    let embedder = LazyEmbedder {
        dir: paths.model_dir.clone(),
        loaded: OnceLock::new(),
    };
    Ok(Box::new(LawIndex::open(&paths.db, Box::new(embedder))?))
}

/// Opens the law index and installs it at once, without warming it: the launch-path half of
/// start-up, so the first `get_config` already reports statutes.
///
/// # Errors
/// Those of [`open_law_index`]; nothing is installed then.
pub fn open_and_install(
    composition: &crate::composition::Composition,
    paths: &LawPaths,
) -> AppResult<()> {
    composition.install_law(open_law_index(paths)?);
    Ok(())
}
