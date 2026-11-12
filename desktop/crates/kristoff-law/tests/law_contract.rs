//! The real `LawIndex` keeps the `LawSearch` contract (the same suite the fake passes).
//! Ignored by default: it needs `desktop/data/phlaws.db` and the model files; `ci-local.sh`
//! runs it when the db exists. Paths: `KRISTOFF_PHLAWS_DB`, `KRISTOFF_MODEL_DIR`.

use std::path::PathBuf;

use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::LawIndex;
use kristoff_testkit::contract::law_search_contract;

fn path_from_env(key: &str, default_rel: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_rel),
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs desktop/data/phlaws.db and desktop/data/model"]
fn law_index_keeps_the_law_search_contract() {
    let model = path_from_env("KRISTOFF_MODEL_DIR", "../../data/model");
    let db = path_from_env("KRISTOFF_PHLAWS_DB", "../../data/phlaws.db");
    let embedder = CandleEmbedder::load(&model, DevicePreference::MetalThenCpu).unwrap();
    let index = LawIndex::open(&db, Box::new(embedder)).unwrap();

    law_search_contract(&index);
}
