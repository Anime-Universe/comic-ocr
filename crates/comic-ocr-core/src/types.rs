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
        batch_size: usize,
    ) -> Result<Vec<OcrResult>, OcrError>;
}
