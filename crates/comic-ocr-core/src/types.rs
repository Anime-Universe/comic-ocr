use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("Image processing error: {0}")]
    ImageError(String),
    #[error("Inference engine error: {0}")]
    EngineError(String),
    #[error("Tokenizer error: {0}")]
    TokenizerError(String),
    #[error("Invalid input parameter: {0}")]
    InvalidInput(String),
    /// A capability this engine advertises but has not implemented.
    ///
    /// Distinct from `EngineError` on purpose. An engine error says something
    /// went wrong at runtime and a retry might help; this says the code path
    /// does not exist, so no retry ever will. Returning a plausible-looking
    /// value here instead — a placeholder string, a default confidence — is how
    /// an unimplemented path gets mistaken for a working one and its output
    /// ends up in a corpus.
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssertionState {
    #[default]
    Candidate,
    Accepted,
    Verified,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    #[default]
    Text,
    SpeechBubble,
    Panel,
    Badge,
    SoundEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub source: String,
    pub engine: String,
    pub model: String,
    pub engine_version: String,
    pub created_at: String,
    pub fields: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionReading {
    pub region_id: String,
    pub text: String,
    pub confidence: Option<f32>,
    pub normalized_bounds: [f32; 4],
    pub kind: RegionKind,
    pub state: AssertionState,
    pub provenance: Option<ProvenanceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLayer {
    pub id: String,
    pub language: String,
    pub kind: String,
    pub regions: Vec<RegionReading>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineType {
    BaseInt8Onnx,
    NanoMobileNet,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrMetadata {
    pub duration_ms: f64,
    pub model_name: String,
    pub engine_type: EngineType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub token_probabilities: Vec<f32>,
    pub metadata: OcrMetadata,
}

pub trait OcrEngine: Send + Sync {
    fn predict(&self, image: &image::DynamicImage) -> Result<OcrResult, OcrError>;
    fn predict_batch(
        &self,
        images: &[image::DynamicImage],
        _batch_size: usize,
    ) -> Result<Vec<OcrResult>, OcrError> {
        images.iter().map(|img| self.predict(img)).collect()
    }
}

