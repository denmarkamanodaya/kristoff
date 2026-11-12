//! `CandleEmbedder`: `intfloat/multilingual-e5-small` sentence embeddings with candle.
//!
//! Plan: `docs/desktop/phases/phase-3-law-index.md` §1.6 and §2, decision D-13 (f16 weights on
//! disk, upcast to f32 at load; never compute in f16 because candle's BERT attention mask turns
//! into `NaN`). Pooling follows the model card: mean over the attention mask, then L2 normalise.
//! Golden vectors: `contracts/fixtures/e5-small-golden.json`.

use std::fmt;
use std::path::Path;

use candle_core::{D, DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

/// Width of one embedding vector.
pub const DIMS: usize = 384;

/// Maximum tokens per input; longer inputs are truncated by the tokenizer.
pub const MAX_TOKENS: usize = 512;

const QUERY_PREFIX: &str = "query: ";
const PASSAGE_PREFIX: &str = "passage: ";

/// Failure while loading the model or embedding text.
#[derive(Debug)]
pub enum EmbedError {
    /// A model file could not be read.
    Io(std::io::Error),
    /// `config.json` is not a valid BERT config.
    Config(serde_json::Error),
    /// The tokenizer failed to load or encode.
    Tokenizer(String),
    /// candle failed while loading weights or running the model.
    Model(candle_core::Error),
}

impl fmt::Display for EmbedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "EMBED_IO: {e}"),
            Self::Config(e) => write!(f, "EMBED_CONFIG: {e}"),
            Self::Tokenizer(e) => write!(f, "EMBED_TOKENIZER: {e}"),
            Self::Model(e) => write!(f, "EMBED_MODEL: {e}"),
        }
    }
}

impl std::error::Error for EmbedError {}

impl From<candle_core::Error> for EmbedError {
    fn from(e: candle_core::Error) -> Self {
        Self::Model(e)
    }
}

/// Which compute device to try first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePreference {
    /// Metal GPU, falling back to the CPU when Metal is unavailable.
    MetalThenCpu,
    /// CPU only.
    Cpu,
}

/// Returns `text` with the e5 query prefix, as the model card requires for search queries.
#[must_use]
pub fn query_text(text: &str) -> String {
    format!("{QUERY_PREFIX}{text}")
}

/// Returns `text` with the e5 passage prefix, as the model card requires for indexed passages.
#[must_use]
pub fn passage_text(text: &str) -> String {
    format!("{PASSAGE_PREFIX}{text}")
}

/// A loaded e5-small model plus its tokenizer, bound to one device.
pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    /// Same vocabulary without padding or truncation, for counting tokens.
    counter: Tokenizer,
    device: Device,
}

impl CandleEmbedder {
    /// Loads `config.json`, `tokenizer.json` and `model.safetensors` from `dir`.
    ///
    /// Weights of any float dtype are upcast to f32 (D-13).
    ///
    /// # Errors
    /// Returns [`EmbedError`] when a file is missing or malformed, or the device cannot be used.
    pub fn load(dir: &Path, preference: DevicePreference) -> Result<Self, EmbedError> {
        let device = pick_device(preference);
        let config_text =
            std::fs::read_to_string(dir.join("config.json")).map_err(EmbedError::Io)?;
        let config: Config = serde_json::from_str(&config_text).map_err(EmbedError::Config)?;
        let counter = Tokenizer::from_file(dir.join("tokenizer.json"))
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        let tokenizer = load_tokenizer(&dir.join("tokenizer.json"))?;
        let weights = std::fs::read(dir.join("model.safetensors")).map_err(EmbedError::Io)?;
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device)?;
        let model = BertModel::load(vb, &config)?;
        Ok(Self {
            model,
            tokenizer,
            counter,
            device,
        })
    }

    /// Names the device in use: `"metal"` or `"cpu"`.
    #[must_use]
    pub fn device_name(&self) -> &'static str {
        if self.device.is_metal() {
            "metal"
        } else {
            "cpu"
        }
    }

    /// Counts the tokens `text` encodes to, including special tokens, before truncation.
    ///
    /// # Errors
    /// Returns [`EmbedError::Tokenizer`] when encoding fails.
    pub fn token_count(&self, text: &str) -> Result<usize, EmbedError> {
        let enc = self
            .counter
            .encode(text, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        Ok(enc.len())
    }

    /// Runs one throw-away query so Metal compiles its kernels now, not on the first search
    /// (the first query of a process otherwise takes seconds). Call it at app start.
    ///
    /// # Errors
    /// Returns [`EmbedError`] when the forward pass fails.
    pub fn warm(&self) -> Result<(), EmbedError> {
        self.embed_queries(&["warm-up"]).map(|_| ())
    }

    /// Embeds search queries (adds the `"query: "` prefix).
    ///
    /// # Errors
    /// Returns [`EmbedError`] when tokenizing or the forward pass fails.
    pub fn embed_queries(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_prefixed(texts.iter().map(|t| query_text(t)).collect())
    }

    /// Embeds passages for the index (adds the `"passage: "` prefix).
    ///
    /// # Errors
    /// Returns [`EmbedError`] when tokenizing or the forward pass fails.
    pub fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_prefixed(texts.iter().map(|t| passage_text(t)).collect())
    }

    /// Embeds texts that already carry their e5 prefix; returns unit-length vectors of [`DIMS`].
    ///
    /// # Errors
    /// Returns [`EmbedError`] when tokenizing or the forward pass fails.
    pub fn embed_prefixed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let enc = self
            .tokenizer
            .encode_batch(texts, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        let ids = enc
            .iter()
            .map(|e| Tensor::new(e.get_ids(), &self.device))
            .collect::<Result<Vec<_>, _>>()?;
        let mask = enc
            .iter()
            .map(|e| Tensor::new(e.get_attention_mask(), &self.device))
            .collect::<Result<Vec<_>, _>>()?;
        let ids = Tensor::stack(&ids, 0)?;
        let mask = Tensor::stack(&mask, 0)?;
        let hidden = self.model.forward(&ids, &ids.zeros_like()?, Some(&mask))?;
        Ok(mean_pool_normalise(&hidden, &mask)?.to_vec2::<f32>()?)
    }
}

fn pick_device(preference: DevicePreference) -> Device {
    match preference {
        DevicePreference::Cpu => Device::Cpu,
        DevicePreference::MetalThenCpu => Device::new_metal(0).unwrap_or(Device::Cpu),
    }
}

fn load_tokenizer(path: &Path) -> Result<Tokenizer, EmbedError> {
    let mut tokenizer =
        Tokenizer::from_file(path).map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    let pad_id = tokenizer
        .token_to_id("<pad>")
        .ok_or_else(|| EmbedError::Tokenizer("tokenizer has no <pad> token".to_owned()))?;
    tokenizer.with_padding(Some(PaddingParams {
        pad_id,
        pad_token: "<pad>".into(),
        ..Default::default()
    }));
    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: MAX_TOKENS,
            ..Default::default()
        }))
        .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    Ok(tokenizer)
}

fn mean_pool_normalise(hidden: &Tensor, mask: &Tensor) -> candle_core::Result<Tensor> {
    let mask = mask.to_dtype(DType::F32)?.unsqueeze(2)?;
    let summed = hidden.broadcast_mul(&mask)?.sum(1)?;
    let mean = summed.broadcast_div(&mask.sum(1)?)?;
    let norm = mean.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    mean.broadcast_div(&norm)
}
