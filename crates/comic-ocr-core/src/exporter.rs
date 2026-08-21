use crate::types::*;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilter {
    pub min_confidence: f32,
    pub dataset_class: DatasetClass,
    pub language: Option<String>,
    pub min_crop_px: u32,
}

impl Default for ExportFilter {
    fn default() -> Self {
        Self {
            min_confidence: 0.0,
            dataset_class: DatasetClass::Gold,
            language: None,
            min_crop_px: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetClass {
    Silver,
    Gold,
    Evaluation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetSplit {
    Train,
    Validation,
    Test,
}

/// Immutable source facts shared by every pair in one page export.
#[derive(Debug, Clone, Copy)]
pub struct ExportContext<'a> {
    pub publication_id: &'a str,
    pub item_id: &'a str,
    pub page_digest: &'a str,
    pub envelope_digest: &'a str,
    pub reviewed_by: Option<&'a str>,
    pub rights_grant_id: &'a str,
    /// A publication/work-level lineage group used to keep related pages out
    /// of different train/evaluation partitions.
    pub split_group: &'a str,
    pub split: DatasetSplit,
    pub direction: &'a str,
    pub source: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportReport {
    pub pairs_written: usize,
    pub skipped_candidate: usize,
    pub skipped_rejected: usize,
    pub skipped_too_small: usize,
    pub skipped_no_geometry: usize,
    pub skipped_low_confidence: usize,
    pub skipped_missing_confidence: usize,
    pub skipped_empty_label: usize,
    pub skipped_wrong_class: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairRecord {
    pub crop: String,
    pub text: String,
    pub empty_is_intentional: bool,
    pub language: String,
    pub direction: String,
    pub source: String,
    pub provenance: TrainingPairProvenance,
    pub geometry: TrainingPairGeometry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairProvenance {
    pub publication_id: String,
    pub item_id: String,
    pub region_id: String,
    pub page_digest: String,
    pub envelope_digest: String,
    pub state: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetManifestRecord {
    pub record: String,
    pub record_digest: String,
    pub crop: String,
    pub crop_digest: String,
    pub region_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetManifest {
    pub version: String,
    pub policy_version: String,
    pub manifest_digest: String,
    pub dataset_class: DatasetClass,
    pub split: DatasetSplit,
    pub split_group: String,
    pub source: String,
    pub rights_grant_id: String,
    pub publication_id: String,
    pub item_id: String,
    pub page_digest: String,
    pub envelope_digest: String,
    pub reviewed_by: Option<String>,
    pub records: Vec<DatasetManifestRecord>,
    pub report: ExportReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairGeometry {
    pub normalized_bounds: Vec<TrainingPairPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPairPoint {
    pub x: f32,
    pub y: f32,
}

fn validate_context(context: &ExportContext<'_>, filter: &ExportFilter) -> Result<(), String> {
    if !filter.min_confidence.is_finite() || !(0.0..=1.0).contains(&filter.min_confidence) {
        return Err("min_confidence must be between 0 and 1".to_string());
    }
    if filter.min_crop_px == 0 {
        return Err("min_crop_px must be greater than zero".to_string());
    }
    if !matches!(context.direction, "ttb" | "ltr" | "rtl") {
        return Err("direction must be ttb, ltr, or rtl".to_string());
    }
    if !matches!(context.source, "own-corpus" | "manga109s" | "synthetic") {
        return Err("source is not permitted by schemas/training_pair.json".to_string());
    }
    for (name, value) in [
        ("publication_id", context.publication_id),
        ("item_id", context.item_id),
        ("page_digest", context.page_digest),
        ("envelope_digest", context.envelope_digest),
        ("rights_grant_id", context.rights_grant_id),
        ("split_group", context.split_group),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} is required for a dataset manifest"));
        }
    }
    match (filter.dataset_class, context.split) {
        (DatasetClass::Evaluation, DatasetSplit::Test)
        | (
            DatasetClass::Silver | DatasetClass::Gold,
            DatasetSplit::Train | DatasetSplit::Validation,
        ) => {}
        _ => {
            return Err(
                "evaluation must use test; silver/gold must use train or validation".to_string(),
            );
        }
    }
    if matches!(
        filter.dataset_class,
        DatasetClass::Gold | DatasetClass::Evaluation
    ) && context.reviewed_by.is_none()
    {
        return Err("gold and evaluation exports require reviewed_by".to_string());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn state_in_class(state: AssertionState, class: DatasetClass) -> bool {
    matches!(
        (state, class),
        (AssertionState::Candidate, DatasetClass::Silver)
            | (
                AssertionState::Accepted | AssertionState::Verified,
                DatasetClass::Gold
            )
            | (AssertionState::Verified, DatasetClass::Evaluation)
    )
}

fn crop_box(bounds: [f32; 4], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let [left, top, right, bottom] = bounds;
    if bounds
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || right <= left
        || bottom <= top
    {
        return None;
    }
    let x = (left * width as f32).floor() as u32;
    let y = (top * height as f32).floor() as u32;
    let right_px = (right * width as f32).ceil().min(width as f32) as u32;
    let bottom_px = (bottom * height as f32).ceil().min(height as f32) as u32;
    let crop_width = right_px.saturating_sub(x);
    let crop_height = bottom_px.saturating_sub(y);
    (crop_width > 0 && crop_height > 0).then_some((x, y, crop_width, crop_height))
}

fn geometry(bounds: [f32; 4]) -> TrainingPairGeometry {
    let [left, top, right, bottom] = bounds;
    TrainingPairGeometry {
        normalized_bounds: vec![
            TrainingPairPoint { x: left, y: top },
            TrainingPairPoint { x: right, y: top },
            TrainingPairPoint {
                x: right,
                y: bottom,
            },
            TrainingPairPoint { x: left, y: bottom },
        ],
    }
}

/// Export real PNG crops and records from one canonical page's text layers.
///
/// The caller must resolve the immutable page, envelope, rights grant and split
/// assignment first. This function writes a JCS-canonical, SHA-256-addressed
/// manifest and refuses mixed classes or duplicate crops.
pub fn export_pairs(
    context: &ExportContext<'_>,
    page: &DynamicImage,
    text_layers: &[TextLayer],
    filter: &ExportFilter,
    output_dir: &Path,
) -> Result<ExportReport, String> {
    validate_context(context, filter)?;
    for layer in text_layers {
        if !matches!(layer.language.as_str(), "ja" | "en") {
            return Err(format!(
                "language '{}' is not permitted by schemas/training_pair.json",
                layer.language
            ));
        }
    }
    if output_dir.exists()
        && fs::read_dir(output_dir)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err(format!(
            "export output must be empty: {}",
            output_dir.display()
        ));
    }
    let mut report = ExportReport::default();
    let mut manifest_records = Vec::new();
    let mut crop_digests = HashSet::new();
    let crops_dir = output_dir.join("crops");
    fs::create_dir_all(&crops_dir).map_err(|error| error.to_string())?;

    for (layer_index, layer) in text_layers.iter().enumerate() {
        if let Some(target_lang) = &filter.language
            && &layer.language != target_lang
        {
            continue;
        }
        for (region_index, reading) in layer.regions.iter().enumerate() {
            if reading.state == AssertionState::Rejected {
                report.skipped_rejected += 1;
                continue;
            }
            if !state_in_class(reading.state, filter.dataset_class) {
                if reading.state == AssertionState::Candidate {
                    report.skipped_candidate += 1;
                }
                report.skipped_wrong_class += 1;
                continue;
            }
            if reading.text.is_empty() && !reading.empty_is_intentional {
                report.skipped_empty_label += 1;
                continue;
            }
            let Some(pair_confidence) = reading.confidence else {
                report.skipped_missing_confidence += 1;
                continue;
            };
            if !pair_confidence.is_finite() || !(0.0..=1.0).contains(&pair_confidence) {
                report.skipped_missing_confidence += 1;
                continue;
            }
            if pair_confidence < filter.min_confidence {
                report.skipped_low_confidence += 1;
                continue;
            }
            let Some((x, y, crop_width, crop_height)) =
                crop_box(reading.normalized_bounds, page.width(), page.height())
            else {
                report.skipped_no_geometry += 1;
                continue;
            };
            if crop_width < filter.min_crop_px || crop_height < filter.min_crop_px {
                report.skipped_too_small += 1;
                continue;
            }

            let stem = format!("pair_{layer_index:04}_{region_index:06}");
            let crop_relative = format!("crops/{stem}.png");
            let crop_path = crops_dir.join(format!("{stem}.png"));
            page.crop_imm(x, y, crop_width, crop_height)
                .save(&crop_path)
                .map_err(|error| error.to_string())?;
            let crop_bytes = fs::read(&crop_path).map_err(|error| error.to_string())?;
            let crop_digest = sha256(&crop_bytes);
            if !crop_digests.insert(crop_digest.clone()) {
                return Err(format!("duplicate crop digest {crop_digest}"));
            }
            let state = match reading.state {
                AssertionState::Candidate => "candidate",
                AssertionState::Accepted => "accepted",
                AssertionState::Verified => "verified",
                AssertionState::Rejected => unreachable!(),
            };
            let record = TrainingPairRecord {
                crop: crop_relative.clone(),
                text: reading.text.clone(),
                empty_is_intentional: reading.empty_is_intentional,
                language: layer.language.clone(),
                direction: context.direction.to_string(),
                source: context.source.to_string(),
                provenance: TrainingPairProvenance {
                    publication_id: context.publication_id.to_string(),
                    item_id: context.item_id.to_string(),
                    region_id: reading.region_id.clone(),
                    page_digest: context.page_digest.to_string(),
                    envelope_digest: context.envelope_digest.to_string(),
                    state: state.to_string(),
                    confidence: pair_confidence,
                    reviewed_by: context.reviewed_by.map(str::to_string),
                },
                geometry: geometry(reading.normalized_bounds),
            };
            let record_path = format!("{stem}.json");
            let json = serde_jcs::to_vec(&record).map_err(|error| error.to_string())?;
            let record_digest = sha256(&json);
            fs::write(output_dir.join(&record_path), &json).map_err(|error| error.to_string())?;
            manifest_records.push(DatasetManifestRecord {
                record: record_path,
                record_digest,
                crop: crop_relative,
                crop_digest,
                region_id: reading.region_id.clone(),
            });
            report.pairs_written += 1;
        }
    }
    let report_json = serde_jcs::to_vec(&report).map_err(|error| error.to_string())?;
    fs::write(output_dir.join("export_report.json"), report_json)
        .map_err(|error| error.to_string())?;
    let mut manifest = DatasetManifest {
        version: "1.0".to_string(),
        policy_version: "curation-training-v1".to_string(),
        manifest_digest: String::new(),
        dataset_class: filter.dataset_class,
        split: context.split,
        split_group: context.split_group.to_string(),
        source: context.source.to_string(),
        rights_grant_id: context.rights_grant_id.to_string(),
        publication_id: context.publication_id.to_string(),
        item_id: context.item_id.to_string(),
        page_digest: context.page_digest.to_string(),
        envelope_digest: context.envelope_digest.to_string(),
        reviewed_by: context.reviewed_by.map(str::to_string),
        records: manifest_records,
        report: report.clone(),
    };
    let unsigned = serde_jcs::to_vec(&manifest).map_err(|error| error.to_string())?;
    manifest.manifest_digest = sha256(&unsigned);
    let manifest_json = serde_jcs::to_vec(&manifest).map_err(|error| error.to_string())?;
    fs::write(output_dir.join("dataset-manifest.json"), manifest_json)
        .map_err(|error| error.to_string())?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    const TRAINING_PAIR_SCHEMA: &str = include_str!("../../../schemas/training_pair.json");
    const DATASET_MANIFEST_SCHEMA: &str = include_str!("../../../schemas/dataset_manifest.json");

    fn assert_schema_valid(schema_source: &str, instance: &serde_json::Value) {
        let schema: serde_json::Value =
            serde_json::from_str(schema_source).expect("checked-in schema parses");
        let validator = jsonschema::validator_for(&schema).expect("checked-in schema compiles");
        let violations: Vec<String> = validator
            .iter_errors(instance)
            .map(|violation| format!("{violation} at {}", violation.instance_path))
            .collect();
        assert!(
            violations.is_empty(),
            "exported JSON violates its checked-in schema:\n{}",
            violations.join("\n")
        );
    }

    fn assert_export_conforms(output: &Path) {
        let manifest_json: serde_json::Value = serde_json::from_slice(
            &fs::read(output.join("dataset-manifest.json")).expect("manifest exists"),
        )
        .expect("manifest is JSON");
        assert_schema_valid(DATASET_MANIFEST_SCHEMA, &manifest_json);
        for entry in manifest_json["records"]
            .as_array()
            .expect("manifest records")
        {
            let record_path = entry["record"].as_str().expect("record path");
            let record_json: serde_json::Value = serde_json::from_slice(
                &fs::read(output.join(record_path)).expect("training record exists"),
            )
            .expect("training record is JSON");
            assert_schema_valid(TRAINING_PAIR_SCHEMA, &record_json);
        }
    }

    fn reading(id: &str, state: AssertionState) -> RegionReading {
        RegionReading {
            region_id: id.to_string(),
            text: "テスト".to_string(),
            confidence: Some(0.85),
            normalized_bounds: [0.1, 0.1, 0.5, 0.5],
            kind: RegionKind::Text,
            state,
            empty_is_intentional: false,
            provenance: None,
        }
    }

    fn context<'a>(reviewed_by: Option<&'a str>) -> ExportContext<'a> {
        ExportContext {
            publication_id: "pub_1",
            item_id: "item_1",
            page_digest: "sha256:page_1",
            envelope_digest: "sha256:envelope_1",
            reviewed_by,
            rights_grant_id: "grant_1",
            split_group: "work_1",
            split: DatasetSplit::Train,
            direction: "ttb",
            source: "own-corpus",
        }
    }

    fn output_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "comic_ocr_exporter_{}_{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn gold_default_excludes_candidates_and_writes_real_crop() {
        let output = output_dir("gold");
        let _ = fs::remove_dir_all(&output);
        let page = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let layers = vec![TextLayer {
            id: "tl-1".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![
                reading("candidate", AssertionState::Candidate),
                reading("accepted", AssertionState::Accepted),
            ],
        }];
        let report = export_pairs(
            &context(Some("op_test")),
            &page,
            &layers,
            &ExportFilter::default(),
            &output,
        )
        .unwrap();
        assert_eq!(report.pairs_written, 1);
        assert_eq!(report.skipped_candidate, 1);
        assert!(output.join("crops/pair_0000_000001.png").is_file());
        let json = fs::read_to_string(output.join("pair_0000_000001.json")).unwrap();
        let record_json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let record: TrainingPairRecord = serde_json::from_value(record_json).unwrap();
        assert_eq!(record.provenance.state, "accepted");
        assert_eq!(record.provenance.reviewed_by.as_deref(), Some("op_test"));
        assert_eq!(record.geometry.normalized_bounds.len(), 4);
        let manifest_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(output.join("dataset-manifest.json")).unwrap(),
        )
        .unwrap();
        let manifest: DatasetManifest = serde_json::from_value(manifest_json).unwrap();
        assert_eq!(manifest.dataset_class, DatasetClass::Gold);
        assert_eq!(manifest.records.len(), 1);
        assert!(manifest.manifest_digest.starts_with("sha256:"));
        assert_export_conforms(&output);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn candidate_export_is_explicit_opt_in() {
        let output = output_dir("silver");
        let _ = fs::remove_dir_all(&output);
        let page = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let layers = vec![TextLayer {
            id: "tl-1".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![reading("candidate", AssertionState::Candidate)],
        }];
        let filter = ExportFilter {
            dataset_class: DatasetClass::Silver,
            ..ExportFilter::default()
        };
        let report = export_pairs(&context(None), &page, &layers, &filter, &output).unwrap();
        assert_eq!(report.pairs_written, 1);
        assert_export_conforms(&output);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn empty_small_and_unreviewed_regions_fail_closed() {
        let output = output_dir("gates");
        let _ = fs::remove_dir_all(&output);
        let page = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let mut empty = reading("empty", AssertionState::Accepted);
        empty.text.clear();
        let mut small = reading("small", AssertionState::Accepted);
        small.normalized_bounds = [0.0, 0.0, 0.01, 0.01];
        let layer = TextLayer {
            id: "tl-1".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![empty, small],
        };
        let reviewed = export_pairs(
            &context(Some("op_test")),
            &page,
            &[layer.clone()],
            &ExportFilter::default(),
            &output,
        )
        .unwrap();
        assert_eq!(reviewed.skipped_empty_label, 1);
        assert_eq!(reviewed.skipped_too_small, 1);
        let unreviewed_output = output_dir("unreviewed");
        let _ = fs::remove_dir_all(&unreviewed_output);
        let unreviewed = export_pairs(
            &context(None),
            &page,
            &[layer],
            &ExportFilter::default(),
            &unreviewed_output,
        )
        .expect_err("gold needs reviewer attribution");
        assert!(unreviewed.contains("reviewed_by"));
        let _ = fs::remove_dir_all(output);
        let _ = fs::remove_dir_all(unreviewed_output);
    }

    #[test]
    fn evaluation_is_verified_and_test_only() {
        let output = output_dir("evaluation");
        let _ = fs::remove_dir_all(&output);
        let page = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let layer = TextLayer {
            id: "tl-1".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![
                reading("accepted", AssertionState::Accepted),
                reading("verified", AssertionState::Verified),
            ],
        };
        let mut evaluation = context(Some("op_second"));
        evaluation.split = DatasetSplit::Test;
        let filter = ExportFilter {
            dataset_class: DatasetClass::Evaluation,
            ..ExportFilter::default()
        };
        let report = export_pairs(&evaluation, &page, &[layer], &filter, &output).unwrap();
        assert_eq!(report.pairs_written, 1);
        assert_eq!(report.skipped_wrong_class, 1);
        assert_export_conforms(&output);
        let _ = fs::remove_dir_all(output);
    }

    #[test]
    fn manifest_identity_is_deterministic_and_binds_source_identity() {
        let first = output_dir("manifest_first");
        let second = output_dir("manifest_second");
        let changed = output_dir("manifest_changed");
        for path in [&first, &second, &changed] {
            let _ = fs::remove_dir_all(path);
        }
        let page = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let layer = TextLayer {
            id: "tl-1".to_string(),
            language: "ja".to_string(),
            kind: "transcription".to_string(),
            regions: vec![reading("accepted", AssertionState::Accepted)],
        };
        let base = context(Some("op_test"));
        export_pairs(
            &base,
            &page,
            &[layer.clone()],
            &ExportFilter::default(),
            &first,
        )
        .unwrap();
        assert!(
            export_pairs(
                &base,
                &page,
                &[layer.clone()],
                &ExportFilter::default(),
                &first,
            )
            .expect_err("an immutable output root cannot be overwritten")
            .contains("must be empty")
        );
        export_pairs(
            &base,
            &page,
            &[layer.clone()],
            &ExportFilter::default(),
            &second,
        )
        .unwrap();
        let first_manifest: DatasetManifest =
            serde_json::from_slice(&fs::read(first.join("dataset-manifest.json")).unwrap())
                .unwrap();
        let second_manifest: DatasetManifest =
            serde_json::from_slice(&fs::read(second.join("dataset-manifest.json")).unwrap())
                .unwrap();
        assert_eq!(
            first_manifest.manifest_digest,
            second_manifest.manifest_digest
        );

        let mut changed_context = context(Some("op_test"));
        changed_context.page_digest = "sha256:different-page";
        export_pairs(
            &changed_context,
            &page,
            &[layer],
            &ExportFilter::default(),
            &changed,
        )
        .unwrap();
        let changed_manifest: DatasetManifest =
            serde_json::from_slice(&fs::read(changed.join("dataset-manifest.json")).unwrap())
                .unwrap();
        assert_ne!(
            first_manifest.manifest_digest,
            changed_manifest.manifest_digest
        );
        for path in [first, second, changed] {
            let _ = fs::remove_dir_all(path);
        }
    }
}
