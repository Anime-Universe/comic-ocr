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
    pub brier_weighted_confidence: f32,
    pub candidates: Vec<OcrResult>,
    pub disagreement_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineBrierCalibration {
    pub engine_type: String,
    pub brier_score: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisagreementReport {
    pub region_id: Option<String>,
    pub cer_divergence: f32,
    pub candidate_texts: Vec<String>,
    pub requires_human_review: bool,
}

pub struct PanelEvaluator {
    engines: Vec<Box<dyn OcrEngine>>,
    brier_scores: Vec<f32>,
    invalidation_threshold: f32,
    disagreement_threshold: f32,
}

impl PanelEvaluator {
    pub fn new(engines: Vec<Box<dyn OcrEngine>>, invalidation_threshold: f32) -> Self {
        let brier_scores = vec![0.10; engines.len()];
        Self {
            engines,
            brier_scores,
            invalidation_threshold,
            disagreement_threshold: 0.20,
        }
    }

    pub fn with_brier_scores(
        engines: Vec<Box<dyn OcrEngine>>,
        brier_scores: Vec<f32>,
        invalidation_threshold: f32,
    ) -> Self {
        Self {
            engines,
            brier_scores,
            invalidation_threshold,
            disagreement_threshold: 0.20,
        }
    }

    /// Calculates Brier-calibrated weight w_i = exp(-Brier_i)
    pub fn calculate_brier_weight(brier_score: f32) -> f32 {
        (-brier_score.clamp(0.0, 1.0)).exp()
    }

    /// Detects cross-engine divergence (uncorrelated reader disagreement)
    pub fn detect_disagreement(
        candidates: &[OcrResult],
        threshold: f32,
    ) -> Option<DisagreementReport> {
        if candidates.len() < 2 {
            return None;
        }

        let first = &candidates[0].text;
        let second = &candidates[1].text;

        if first == second {
            return Some(DisagreementReport {
                region_id: None,
                cer_divergence: 0.0,
                candidate_texts: candidates.iter().map(|c| c.text.clone()).collect(),
                requires_human_review: false,
            });
        }

        // Levenshtein CER divergence calculation
        let first_chars: Vec<char> = first.chars().collect();
        let second_chars: Vec<char> = second.chars().collect();
        let max_len = first_chars.len().max(second_chars.len());
        if max_len == 0 {
            return None;
        }

        let dist = levenshtein_dist(&first_chars, &second_chars) as f32;
        let cer_divergence = dist / (max_len as f32);
        let requires_human_review = cer_divergence >= threshold;

        Some(DisagreementReport {
            region_id: None,
            cer_divergence,
            candidate_texts: candidates.iter().map(|c| c.text.clone()).collect(),
            requires_human_review,
        })
    }

    pub fn evaluate(&self, image: &image::DynamicImage) -> Result<PdpDecision, PdpError> {
        if self.engines.is_empty() {
            return Err(PdpError::EvaluationError(
                "No OCR engines registered in panel".into(),
            ));
        }

        let mut candidates = Vec::with_capacity(self.engines.len());
        let mut weighted_conf_sum = 0.0f32;
        let mut weight_sum = 0.0f32;

        for (idx, engine) in self.engines.iter().enumerate() {
            if let Ok(res) = engine.predict(image) {
                let brier = self.brier_scores.get(idx).copied().unwrap_or(0.10);
                let w = Self::calculate_brier_weight(brier);

                weighted_conf_sum += res.confidence * w;
                weight_sum += w;
                candidates.push(res);
            }
        }

        if candidates.is_empty() {
            return Err(PdpError::EvaluationError(
                "All panel engines failed prediction".into(),
            ));
        }

        // Select candidate with highest confidence
        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best = candidates[0].clone();
        let brier_weighted_confidence = if weight_sum > 0.0 {
            weighted_conf_sum / weight_sum
        } else {
            best.confidence
        };

        let is_validated = brier_weighted_confidence >= self.invalidation_threshold;
        let disagreement = Self::detect_disagreement(&candidates, self.disagreement_threshold);
        let disagreement_detected = disagreement
            .as_ref()
            .map(|d| d.requires_human_review)
            .unwrap_or(false);

        Ok(PdpDecision {
            selected_text: best.text,
            confidence: best.confidence,
            is_validated,
            brier_weighted_confidence,
            candidates,
            disagreement_detected,
        })
    }
}

fn levenshtein_dist(a: &[char], b: &[char]) -> usize {
    let mut distances = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in distances.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in distances[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            distances[i][j] = (distances[i - 1][j] + 1)
                .min(distances[i][j - 1] + 1)
                .min(distances[i - 1][j - 1] + cost);
        }
    }
    distances[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use comic_ocr_core::{EngineType, OcrMetadata};

    struct MockEngine {
        output: String,
        conf: f32,
    }

    impl OcrEngine for MockEngine {
        fn predict(
            &self,
            _image: &image::DynamicImage,
        ) -> Result<OcrResult, comic_ocr_core::OcrError> {
            Ok(OcrResult {
                text: self.output.clone(),
                confidence: self.conf,
                token_probabilities: vec![self.conf],
                metadata: OcrMetadata {
                    duration_ms: 5.0,
                    model_name: "Mock".into(),
                    engine_type: EngineType::Fallback,
                },
            })
        }
    }

    #[test]
    fn test_brier_calibration_and_disagreement_detection() {
        let e1 = Box::new(MockEngine {
            output: "こんにちは".into(),
            conf: 0.95,
        });
        let e2 = Box::new(MockEngine {
            output: "こんばんは".into(), // Disagreement
            conf: 0.80,
        });

        let evaluator = PanelEvaluator::with_brier_scores(vec![e1, e2], vec![0.05, 0.20], 0.70);
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(10, 10));
        let decision = evaluator.evaluate(&img).expect("eval failed");

        assert_eq!(decision.selected_text, "こんにちは");
        assert!(decision.brier_weighted_confidence > 0.85);
        assert!(decision.disagreement_detected);
    }
}
