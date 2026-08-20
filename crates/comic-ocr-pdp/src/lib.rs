use comic_ocr_core::{OcrEngine, OcrResult};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdpError {
    #[error("Panel evaluation error: {0}")]
    EvaluationError(String),
    #[error("Invalidation trigger activated: confidence {0} below threshold {1}")]
    Invalidated(f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdpDecision {
    pub selected_text: String,
    pub confidence: f32,
    pub is_validated: bool,
    pub candidates: Vec<OcrResult>,
}

pub struct PanelEvaluator {
    engines: Vec<Box<dyn OcrEngine>>,
    invalidation_threshold: f32,
}

impl PanelEvaluator {
    pub fn new(engines: Vec<Box<dyn OcrEngine>>, invalidation_threshold: f32) -> Self {
        Self {
            engines,
            invalidation_threshold,
        }
    }

    pub fn evaluate(&self, image: &image::DynamicImage) -> Result<PdpDecision, PdpError> {
        if self.engines.is_empty() {
            return Err(PdpError::EvaluationError(
                "No OCR engines registered in panel".into(),
            ));
        }

        let mut candidates = Vec::with_capacity(self.engines.len());
        for engine in &self.engines {
            if let Ok(res) = engine.predict(image) {
                candidates.push(res);
            }
        }

        if candidates.is_empty() {
            return Err(PdpError::EvaluationError(
                "All panel engines failed prediction".into(),
            ));
        }

        // ACS Discounting & Selection: Select candidate with highest confidence
        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let best = candidates[0].clone();
        let is_validated = best.confidence >= self.invalidation_threshold;

        Ok(PdpDecision {
            selected_text: best.text,
            confidence: best.confidence,
            is_validated,
            candidates,
        })
    }
}
