use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceSource {
    Ocr,
    Vision,
    Geometry,
    Metadata,
    LanguageModel,
    Human,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisEvidence {
    pub source: ProvenanceSource,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationStatus {
    Confirmed,
    Unverified,
    Ambiguous,
    Contradicted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticAssertion<T> {
    pub value: T,
    pub confidence: f32,
    #[serde(default)]
    pub evidence: Vec<AnalysisEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_status: Option<ValidationStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadingOrderViolationReason {
    HorizontalOrder,
    VerticalOrder,
    BandOrder,
    NestedTextOrder,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingOrderViolation {
    pub first_id: String,
    pub second_id: String,
    pub reason: ReadingOrderViolationReason,
    pub confidence: f32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingOrderValidation {
    pub declared_direction: String,
    pub inferred_direction: Option<String>,
    pub status: String,
    pub violations: Vec<ReadingOrderViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContradictionKind {
    TextVsText,
    TextVsVisual,
    VisualVsVisual,
    MetadataVsStructure,
    DerivedVsSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnomalyNature {
    AuthoringError,
    CharacterPerceivedDiscontinuity,
    TemporalAnomaly,
    NarrativeDevice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticConflict {
    pub conflict_type: String,
    pub severity: String,
    pub objects: Vec<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contradiction_kind: Option<ContradictionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomaly_nature: Option<AnomalyNature>,
}

/// Evaluates cardinality invariants (e.g. page_metadata.total_panels == panels.len()).
pub fn validate_cardinality_invariants(
    declared_total_panels: usize,
    actual_panels_len: usize,
    panel_ids: &[String],
) -> Vec<SemanticConflict> {
    let mut conflicts = Vec::new();

    if declared_total_panels != actual_panels_len {
        conflicts.push(SemanticConflict {
            conflict_type: "cardinality-mismatch".to_string(),
            severity: "error".to_string(),
            objects: vec![
                "page_metadata.total_panels".to_string(),
                "panels".to_string(),
            ],
            message: format!(
                "Document metadata declares total_panels: {}, but panels array contains {} elements.",
                declared_total_panels, actual_panels_len
            ),
            contradiction_kind: Some(ContradictionKind::MetadataVsStructure),
            anomaly_nature: Some(AnomalyNature::AuthoringError),
        });
    }

    // Check for duplicate panel IDs
    let mut seen_ids = std::collections::HashSet::new();
    for id in panel_ids {
        if !seen_ids.insert(id) {
            conflicts.push(SemanticConflict {
                conflict_type: "duplicate-panel-id".to_string(),
                severity: "error".to_string(),
                objects: vec![id.clone()],
                message: format!("Duplicate panel ID '{}' found in scene graph.", id),
                contradiction_kind: Some(ContradictionKind::MetadataVsStructure),
                anomaly_nature: Some(AnomalyNature::AuthoringError),
            });
        }
    }

    conflicts
}

/// Evaluates spatial containment of child text regions within parent panel bounds.
pub fn validate_spatial_containment(
    panel_bounds: [f32; 4], // [x, y, w, h]
    text_bounds: [f32; 4],  // [x, y, w, h]
    region_id: &str,
    panel_id: &str,
    tolerance_px: f32,
) -> Option<SemanticConflict> {
    let panel_x2 = panel_bounds[0] + panel_bounds[2];
    let panel_y2 = panel_bounds[1] + panel_bounds[3];

    let text_x2 = text_bounds[0] + text_bounds[2];
    let text_y2 = text_bounds[1] + text_bounds[3];

    // Calculate how far outside text region extends beyond panel bounds
    let overflow_left = (panel_bounds[0] - text_bounds[0]).max(0.0);
    let overflow_right = (text_x2 - panel_x2).max(0.0);
    let overflow_top = (panel_bounds[1] - text_bounds[1]).max(0.0);
    let overflow_bottom = (text_y2 - panel_y2).max(0.0);

    let max_overflow = overflow_left
        .max(overflow_right)
        .max(overflow_top)
        .max(overflow_bottom);

    if max_overflow > tolerance_px {
        Some(SemanticConflict {
            conflict_type: "spatial-containment-violation".to_string(),
            severity: "error".to_string(),
            objects: vec![region_id.to_string(), panel_id.to_string()],
            message: format!(
                "Text region '{}' extends {:.1}px outside parent panel '{}' bounds (tolerance: {:.1}px).",
                region_id, max_overflow, panel_id, tolerance_px
            ),
            contradiction_kind: Some(ContradictionKind::TextVsVisual),
            anomaly_nature: Some(AnomalyNature::AuthoringError),
        })
    } else {
        None
    }
}

/// Evaluates spatial panel sequence against declared reading direction (RTL or LTR).
pub fn validate_panel_reading_order(
    panels: &[(String, [f32; 4], usize)], // (id, [x, y, w, h], reading_order)
    declared_direction: &str,
) -> ReadingOrderValidation {
    let mut violations = Vec::new();

    if declared_direction == "right_to_left" || declared_direction == "rtl" {
        // Group panels into horizontal row bands (same vertical band if Y centers within 50px)
        for i in 0..panels.len() {
            for j in (i + 1)..panels.len() {
                let (id_a, bounds_a, order_a) = &panels[i];
                let (id_b, bounds_b, order_b) = &panels[j];

                let y_center_a = bounds_a[1] + bounds_a[3] / 2.0;
                let y_center_b = bounds_b[1] + bounds_b[3] / 2.0;

                // Same horizontal row band
                if (y_center_a - y_center_b).abs() < 60.0 {
                    let x_left_a = bounds_a[0];
                    let x_left_b = bounds_b[0];

                    // For RTL, right panel (higher x) MUST have earlier reading order (smaller index)
                    if x_left_a < x_left_b && order_a < order_b {
                        violations.push(ReadingOrderViolation {
                            first_id: id_a.clone(),
                            second_id: id_b.clone(),
                            reason: ReadingOrderViolationReason::HorizontalOrder,
                            confidence: 0.98,
                            message: format!(
                                "Page declares RTL manga reading, but panel {} (x={}) precedes panel {} (x={}) in left-to-right sequence.",
                                id_a, x_left_a, id_b, x_left_b
                            ),
                        });
                    }
                }
            }
        }
    }

    let status = if violations.is_empty() {
        "consistent".to_string()
    } else {
        "contradiction".to_string()
    };

    ReadingOrderValidation {
        declared_direction: declared_direction.to_string(),
        inferred_direction: Some(declared_direction.to_string()),
        status,
        violations,
    }
}

/// Validates semantic conflicts such as page badge number vs chapter number.
pub fn validate_semantic_roles(
    chapter_number: Option<u32>,
    continuation_chapter: Option<u32>,
    badge_number: Option<u32>,
) -> Vec<SemanticConflict> {
    let mut conflicts = Vec::new();

    #[allow(clippy::collapsible_if)]
    if let (Some(ch), Some(badge)) = (chapter_number, badge_number) {
        if ch == badge && continuation_chapter.is_some_and(|next_ch| next_ch < ch) {
            conflicts.push(SemanticConflict {
                conflict_type: "number-role-conflict".to_string(),
                severity: "error".to_string(),
                objects: vec![
                    "series_info.chapter_number".to_string(),
                    "footer.chapter_number_badge".to_string(),
                ],
                message: format!(
                    "Numeric label {} was classified as chapter_number, but spatial context and continuation text specify next chapter is {}.",
                    badge, continuation_chapter.unwrap()
                ),
                contradiction_kind: Some(ContradictionKind::MetadataVsStructure),
                anomaly_nature: Some(AnomalyNature::AuthoringError),
            });
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cardinality_invariants() {
        let conflicts = validate_cardinality_invariants(10, 13, &["p1".into(), "p2".into()]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, "cardinality-mismatch");
    }

    #[test]
    fn test_validate_spatial_containment_violation() {
        let panel_bounds = [745.0, 860.0, 265.0, 275.0];
        let text_bounds = [875.0, 580.0, 115.0, 110.0];

        let conflict =
            validate_spatial_containment(panel_bounds, text_bounds, "text-7", "panel-7", 50.0);
        assert!(conflict.is_some());
        assert_eq!(
            conflict.unwrap().conflict_type,
            "spatial-containment-violation"
        );
    }
}
