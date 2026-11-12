//! The real `CaseIndex` keeps the `CaseSearch` contract (the same suite the fake passes).
//! Ignored by default: it needs `desktop/data/phcases.db` and the model files; `ci-local.sh`
//! runs it when the db exists. Paths: `KRISTOFF_PHCASES_DB`, `KRISTOFF_MODEL_DIR`.

use std::path::PathBuf;

use kristoff_embed::{CandleEmbedder, DevicePreference};
use kristoff_law::CaseIndex;
use kristoff_testkit::contract::{KnownCase, case_search_contract};

/// A decision `phcases.db` holds (`OmniCorpus` @ bb0626ce): People v. Prajes, 2014, a buy-bust case.
const PRAJES: KnownCase = KnownCase {
    gr_number: "G.R. No. 206770",
    query: "Noel Prajes Alipa Mala buy-bust sale of shabu",
    year: 2014,
    focus: "buy-bust",
};

fn path_from_env(key: &str, default_rel: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_rel),
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs desktop/data/phcases.db and desktop/data/model"]
fn case_index_keeps_the_case_search_contract() {
    let model = path_from_env("KRISTOFF_MODEL_DIR", "../../data/model");
    let db = path_from_env("KRISTOFF_PHCASES_DB", "../../data/phcases.db");
    let embedder = CandleEmbedder::load(&model, DevicePreference::MetalThenCpu).unwrap();
    let index = CaseIndex::open(&db, Box::new(embedder)).unwrap();

    case_search_contract(&index, &PRAJES);
}
