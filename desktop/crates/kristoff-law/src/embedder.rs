//! Binds the candle e5 embedder to the index's [`QueryEmbedder`] seam.

use kristoff_embed::CandleEmbedder;

use crate::QueryEmbedder;

impl QueryEmbedder for CandleEmbedder {
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        let mut vectors = self.embed_queries(&[text]).map_err(|e| e.to_string())?;
        vectors
            .pop()
            .ok_or_else(|| "EMBED_EMPTY: no vector returned".to_owned())
    }

    fn warm(&self) -> Result<(), String> {
        CandleEmbedder::warm(self).map_err(|e| e.to_string())
    }
}
