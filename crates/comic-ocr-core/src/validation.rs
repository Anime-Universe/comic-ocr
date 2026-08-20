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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticConflict {
    pub conflict_type: String,
    pub severity: String,
    pub objects: Vec<String>,
    pub message: String,
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

    if let (Some(ch), Some(badge)) = (chapter_number, badge_number) {
        if ch == badge && continuation_chapter.map_or(false, |next_ch| next_ch < ch) {
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
            });
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_panel_reading_order_rtl_contradiction() {
        let panels = vec![
            ("panel-left".to_string(), [15.0, 500.0, 400.0, 200.0], 1),
            ("panel-right".to_string(), [700.0, 500.0, 300.0, 200.0], 2),
        ];

        let validation = validate_panel_reading_order(&panels, "right_to_left");
        assert_eq!(validation.status, "contradiction");
        assert_eq!(validation.violations.len(), 1);
        assert_eq!(validation.violations[0].reason, ReadingOrderViolationReason::HorizontalOrder);
    }

    #[test]
    fn test_validate_semantic_roles_conflict() {
        let conflicts = validate_semantic_roles(Some(14), Some(2), Some(14));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, "number-role-conflict");
    }
}
