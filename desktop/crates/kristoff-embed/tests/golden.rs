//! Golden test: candle output matches the Python reference vectors (spike S6, D-13).
//! Needs the model files (`scripts/fetch-model.sh`), so it is ignored by default.
//! Paths: `KRISTOFF_MODEL_DIR` (default `data/model`) and `KRISTOFF_E5_GOLDEN`
//! (default `contracts/fixtures/e5-small-golden.json` at the kristoff repo root).

use std::error::Error;
use std::path::PathBuf;

use kristoff_embed::{CandleEmbedder, DIMS, DevicePreference};

#[derive(serde::Deserialize)]
struct Golden {
    sentences: Vec<String>,
    vectors: Vec<Vec<f32>>,
}

fn path_from_env(key: &str, default_rel: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_rel),
        PathBuf::from,
    )
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

fn golden() -> Result<Golden, Box<dyn Error>> {
    let path = path_from_env(
        "KRISTOFF_E5_GOLDEN",
        "../../../contracts/fixtures/e5-small-golden.json",
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn assert_matches_golden(preference: DevicePreference) -> Result<(), Box<dyn Error>> {
    let model = path_from_env("KRISTOFF_MODEL_DIR", "../../data/model");
    let embedder = CandleEmbedder::load(&model, preference)?;
    let golden = golden()?;
    let queries: Vec<&str> = golden.sentences.iter().map(String::as_str).collect();

    let vectors = embedder.embed_queries(&queries)?;

    for (i, (got, want)) in vectors.iter().zip(&golden.vectors).enumerate() {
        assert_eq!(got.len(), DIMS);
        let c = cosine(got, want);
        assert!(
            c >= 0.999,
            "sentence {i} cosine {c} on {}",
            embedder.device_name()
        );
    }
    Ok(())
}

#[test]
#[ignore = "needs data/model from scripts/fetch-model.sh"]
fn query_vectors_match_the_python_reference_on_metal_or_cpu_fallback() {
    assert_matches_golden(DevicePreference::MetalThenCpu).unwrap();
}

#[test]
#[ignore = "needs data/model from scripts/fetch-model.sh"]
fn query_vectors_match_the_python_reference_on_cpu() {
    assert_matches_golden(DevicePreference::Cpu).unwrap();
}
