use crate::validation::AnalysisEvidence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AmbiguityType {
    InsetPanelReadingOrder,
    EnvironmentalSignVsDialogue,
    OccludedSfxBounds,
    NumericLabelClassification,
    AmbiguousSpeakerTail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionStatus {
    Resolved,
    Unresolved,
    HumanReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hypothesis<T> {
    pub value: T,
    pub confidence: f32,
    pub rationale: String,
    #[serde(default)]
    pub evidence: Vec<AnalysisEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbiguousProperty<T> {
    pub primary: Hypothesis<T>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Hypothesis<T>>,
    pub ambiguity_type: AmbiguityType,
    pub resolution_status: ResolutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_delta: Option<f32>,
}

impl<T> AmbiguousProperty<T> {
    pub fn new(
        primary: Hypothesis<T>,
        alternatives: Vec<Hypothesis<T>>,
        ambiguity_type: AmbiguityType,
    ) -> Self {
        let margin_delta = alternatives
            .first()
            .map(|alt| (primary.confidence - alt.confidence).abs());

        let resolution_status = match margin_delta {
            Some(delta) if delta < 0.15 => ResolutionStatus::HumanReviewRequired,
            Some(_) => ResolutionStatus::Resolved,
            None => ResolutionStatus::Resolved,
        };

        Self {
            primary,
            alternatives,
            ambiguity_type,
            resolution_status,
            margin_delta,
        }
    }
}

/// Evaluates speaker attribution ambiguity when a balloon tail points between two candidate speakers.
pub fn evaluate_speaker_attribution(
    balloon_tail_tip: [f32; 2],
    speaker_a: (&str, [f32; 2]), // (id, center_pos)
    speaker_b: (&str, [f32; 2]),
) -> AmbiguousProperty<String> {
    let dist_a = ((balloon_tail_tip[0] - speaker_a.1[0]).powi(2)
        + (balloon_tail_tip[1] - speaker_a.1[1]).powi(2))
    .sqrt();

    let dist_b = ((balloon_tail_tip[0] - speaker_b.1[0]).powi(2)
        + (balloon_tail_tip[1] - speaker_b.1[1]).powi(2))
    .sqrt();

    let total = dist_a + dist_b;
    let prob_a = 1.0 - (dist_a / total);
    let prob_b = 1.0 - (dist_b / total);

    let (primary_speaker, primary_prob, alt_speaker, alt_prob) = if prob_a >= prob_b {
        (speaker_a.0, prob_a, speaker_b.0, prob_b)
    } else {
        (speaker_b.0, prob_b, speaker_a.0, prob_a)
    };

    AmbiguousProperty::new(
        Hypothesis {
            value: primary_speaker.to_string(),
            confidence: primary_prob,
            rationale: format!(
                "Tail tip is {:.1}px from {} vs {:.1}px from {}",
                dist_a, speaker_a.0, dist_b, speaker_b.0
            ),
            evidence: vec![],
        },
        vec![Hypothesis {
            value: alt_speaker.to_string(),
            confidence: alt_prob,
            rationale: "Alternative plausible speaker candidate within tail proximity radius"
                .to_string(),
            evidence: vec![],
        }],
        AmbiguityType::AmbiguousSpeakerTail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speaker_attribution_ambiguity_flagging() {
        // Tail tip is right in the middle between character-a and character-b
        let tail_tip = [500.0, 500.0];
        let speaker_a = ("character-a", [480.0, 500.0]);
        let speaker_b = ("character-b", [520.0, 500.0]);

        let result = evaluate_speaker_attribution(tail_tip, speaker_a, speaker_b);
        assert_eq!(
            result.resolution_status,
            ResolutionStatus::HumanReviewRequired
        );
        assert!(result.margin_delta.unwrap() < 0.15);
    }
}
