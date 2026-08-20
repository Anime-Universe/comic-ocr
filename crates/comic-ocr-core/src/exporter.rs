use crate::types::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Selection & quality filter options for exporting training pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilter {
    /// Minimum composed pair confidence C_pair = C_det * C_trans in [0.0, 1.0].
    pub min_confidence: f32,
    /// Whether candidate assertions are included (weighted by confidence).
    pub include_candidates: bool,
    /// Language track filter ("ja" or "en"). None exports all languages.
    pub language: Option<String>,
    /// Minimum crop width/height in pixels.
    pub min_crop_px: u32,
}

impl Default for ExportFilter {
    fn default() -> Self {
        Self {
            min_confidence: 0.0,
            include_candidates: true,
            language: None,
            min_crop_px: 16,
        }
    }
}

/// Telemetry report returned by `export_pairs`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportReport {
    pub pairs_written: usize,
    pub skipped_candidate: usize,
    pub skipped_rejected: usize,
    pub skipped_too_small: usize,
    pub skipped_no_geometry: usize,
    pub skipped_low_confidence: usize,
}

/// One exported dataset pair matching `schemas/training_pair.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairRecord {
    pub crop: String,
    pub text: String,
    pub empty_is_intentional: bool,
    pub language: String,
    pub direction: String,
    pub source: String,
    pub provenance: TrainingPairProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairProvenance {
    pub publication_id: Option<String>,
    pub item_id: Option<String>,
    pub region_id: String,
    pub envelope_digest: Option<String>,
    pub state: String,
    pub confidence: f32,
    pub reviewed_by: Option<String>,
}

/// Exports (crop, text) training pairs from `TextLayer` semantic envelope resources.
pub fn export_pairs(
    publication_id: Option<&str>,
    item_id: Option<&str>,
    envelope_digest: Option<&str>,
    text_layers: &[TextLayer],
    filter: &ExportFilter,
    output_dir: &Path,
) -> Result<ExportReport, String> {
    let mut report = ExportReport::default();
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    for layer in text_layers {
        if let Some(ref target_lang) = filter.language {
            if &layer.language != target_lang {
                continue;
            }
        }

        for reading in &layer.regions {
            // Enforce The Training Contract: rejected assertions NEVER train
            if reading.state == AssertionState::Rejected {
                report.skipped_rejected += 1;
                continue;
            }

            if reading.state == AssertionState::Candidate && !filter.include_candidates {
                report.skipped_candidate += 1;
                continue;
            }

            // Composed pair confidence (for single-layer extraction, reading confidence)
            let pair_conf = reading.confidence.unwrap_or(0.5).clamp(0.0, 1.0);

            if pair_conf < filter.min_confidence {
                report.skipped_low_confidence += 1;
                continue;
            }

            let state_str = match reading.state {
                AssertionState::Candidate => "candidate",
                AssertionState::Accepted => "accepted",
                AssertionState::Verified => "verified",
                AssertionState::Rejected => unreachable!(),
            };

            let pair_record = TrainingPairRecord {
                crop: format!("crops/{}.png", reading.region_id),
                text: reading.text.clone(),
                empty_is_intentional: reading.text.is_empty(),
                language: layer.language.clone(),
                direction: "ttb".to_string(),
                source: "own-corpus".to_string(),
                provenance: TrainingPairProvenance {
                    publication_id: publication_id.map(|s| s.to_string()),
                    item_id: item_id.map(|s| s.to_string()),
                    region_id: reading.region_id.clone(),
                    envelope_digest: envelope_digest.map(|s| s.to_string()),
                    state: state_str.to_string(),
                    confidence: pair_conf,
                    reviewed_by: None,
                },
            };

            let pair_json = serde_json::to_string_pretty(&pair_record).map_err(|e| e.to_string())?;
            let record_file = output_dir.join(format!("{}_{}.json", layer.id, reading.region_id));
            fs::write(&record_file, pair_json).map_err(|e| e.to_string())?;

            report.pairs_written += 1;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_pairs_training_contract() {
        let text_layers = vec![TextLayer {
            id: "tl-co-001".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![
                RegionReading {
                    region_id: "co-001".to_string(),
                    text: "テスト".to_string(),
                    confidence: Some(0.85),
                    normalized_bounds: [0.1, 0.1, 0.3, 0.4],
                    kind: RegionKind::Text,
                    state: AssertionState::Candidate,
                    provenance: None,
                },
                RegionReading {
                    region_id: "co-002".to_string(),
                    text: "無効".to_string(),
                    confidence: Some(0.90),
                    normalized_bounds: [0.5, 0.5, 0.7, 0.8],
                    kind: RegionKind::Text,
                    state: AssertionState::Rejected, // Must be skipped
                    provenance: None,
                },
            ],
        }];

        let temp_dir = std::env::temp_dir().join("comic_ocr_exporter_test");
        let filter = ExportFilter::default();
        let report = export_pairs(
            Some("pub_1"),
            Some("item_1"),
            Some("digest_1"),
            &text_layers,
            &filter,
            &temp_dir,
        )
        .expect("export failed");

        assert_eq!(report.pairs_written, 1);
        assert_eq!(report.skipped_rejected, 1);

        let exported_json = std::fs::read_to_string(temp_dir.join("tl-co-001_co-001.json")).unwrap();
        let record: TrainingPairRecord = serde_json::from_str(&exported_json).unwrap();
        assert_eq!(record.provenance.confidence, 0.85);
        assert_eq!(record.provenance.state, "candidate");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
