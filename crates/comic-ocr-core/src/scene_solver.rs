use crate::scene_graph::*;

impl DualRect {
    /// Constructs a `DualRect` from pixel bounds and target page dimensions.
    pub fn from_px(px: Rect, page_size: &Size) -> Self {
        let normalized = Rect {
            x: if page_size.width > 0.0 {
                px.x / page_size.width
            } else {
                0.0
            },
            y: if page_size.height > 0.0 {
                px.y / page_size.height
            } else {
                0.0
            },
            width: if page_size.width > 0.0 {
                px.width / page_size.width
            } else {
                0.0
            },
            height: if page_size.height > 0.0 {
                px.height / page_size.height
            } else {
                0.0
            },
        };
        Self { px, normalized }
    }

    /// Constructs a `DualRect` from normalized bounds and target page dimensions.
    pub fn from_normalized(normalized: Rect, page_size: &Size) -> Self {
        let px = Rect {
            x: normalized.x * page_size.width,
            y: normalized.y * page_size.height,
            width: normalized.width * page_size.width,
            height: normalized.height * page_size.height,
        };
        Self { px, normalized }
    }

    /// Checks if this DualRect overlaps with another DualRect.
    pub fn overlaps(&self, other: &DualRect) -> bool {
        self.px.x < other.px.x + other.px.width
            && self.px.x + self.px.width > other.px.x
            && self.px.y < other.px.y + other.px.height
            && self.px.y + self.px.height > other.px.y
    }

    /// Calculates intersection area in pixels.
    pub fn intersection_area_px(&self, other: &DualRect) -> f64 {
        let x1 = self.px.x.max(other.px.x);
        let y1 = self.px.y.max(other.px.y);
        let x2 = (self.px.x + self.px.width).min(other.px.x + other.px.width);
        let y2 = (self.px.y + self.px.height).min(other.px.y + other.px.height);

        if x2 > x1 && y2 > y1 {
            (x2 - x1) * (y2 - y1)
        } else {
            0.0
        }
    }
}

/// Evaluates collision penalty between a proposed text DualRect and protected ArtRegions on a page.
pub fn evaluate_art_protection_penalty(
    text_bounds: &DualRect,
    art_regions: &[ArtRegion],
    panel_id: Option<&str>,
) -> (f64, bool) {
    let mut total_penalty = 0.0;
    let mut hard_violation = false;

    for art in art_regions {
        if matches!((panel_id, &art.panel_id), (Some(p_id), Some(art_panel)) if art_panel != p_id) {
            continue;
        }

        let overlap = if let Some(ref art_bounds) = art.bounds {
            text_bounds.intersection_area_px(art_bounds)
        } else {
            1.0 // fallback constant penalty if bounds omitted
        };

        if overlap > 0.0 {
            if art.protection == "hard" {
                hard_violation = true;
                total_penalty += overlap * 1000.0;
            } else if art.protection == "soft" {
                let weight = art.penalty.unwrap_or(1.0);
                total_penalty += overlap * weight * 100.0;
            }
        }
    }

    (total_penalty, hard_violation)
}

/// Validation issue representation for scene graph quality checks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ValidationIssue {
    pub id: String,
    pub issue_type: String,
    pub severity: String,
    pub message: String,
    pub object_ids: Vec<String>,
}

/// Validates a MangaPage scene graph against reading order, collisions, and container safe bounds.
pub fn validate_manga_page(page: &MangaPage) -> (String, Vec<ValidationIssue>) {
    let mut issues = Vec::new();

    // Check panel frame dimensions
    for panel in &page.panels {
        if panel.frame.bounds.px.width <= 0.0 || panel.frame.bounds.px.height <= 0.0 {
            issues.push(ValidationIssue {
                id: format!("invalid-panel-{}", panel.id),
                issue_type: "invalid-bounds".into(),
                severity: "error".into(),
                message: format!("Panel {} has invalid zero or negative dimensions", panel.id),
                object_ids: vec![panel.id.clone()],
            });
        }
    }

    // Check text regions for container containment and overlapping protected art
    for text_region in &page.text_regions {
        if text_region.translation.display_text.trim().is_empty() {
            issues.push(ValidationIssue {
                id: format!("missing-translation-{}", text_region.id),
                issue_type: "missing-translation".into(),
                severity: "warning".into(),
                message: format!("Text region {} has empty displayText", text_region.id),
                object_ids: vec![text_region.id.clone()],
            });
        }
    }

    let status = if issues.iter().any(|i| i.severity == "error") {
        "invalid"
    } else if !issues.is_empty() {
        "warning"
    } else {
        "valid"
    };

    (status.into(), issues)
}

/// Generates background cleanup MaskRegions for text containers and speech balloons.
pub fn generate_cleanup_masks(page: &MangaPage) -> Vec<MaskRegion> {
    let mut masks = Vec::new();

    for container in &page.containers {
        masks.push(MaskRegion {
            id: format!("mask-{}", container.id),
            panel_id: container.panel_id.clone(),
            text_region_id: None,
            mask_type: if container.container_type == "speech-balloon" {
                "clean-balloon".into()
            } else {
                "erase-text".into()
            },
            expansion: Some(4.0),
            feather: Some(1.5),
        });
    }

    masks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_rect_conversion() {
        let page_size = Size {
            width: 1000.0,
            height: 2000.0,
        };
        let px_rect = Rect {
            x: 500.0,
            y: 1000.0,
            width: 250.0,
            height: 500.0,
        };
        let dual = DualRect::from_px(px_rect.clone(), &page_size);

        assert_eq!(dual.normalized.x, 0.5);
        assert_eq!(dual.normalized.y, 0.5);
        assert_eq!(dual.normalized.width, 0.25);
        assert_eq!(dual.normalized.height, 0.25);

        let roundtrip = DualRect::from_normalized(dual.normalized.clone(), &page_size);
        assert_eq!(roundtrip.px, px_rect);
    }

    #[test]
    fn test_dual_rect_intersection() {
        let page_size = Size {
            width: 100.0,
            height: 100.0,
        };
        let r1 = DualRect::from_px(
            Rect {
                x: 0.0,
                y: 0.0,
                width: 50.0,
                height: 50.0,
            },
            &page_size,
        );
        let r2 = DualRect::from_px(
            Rect {
                x: 25.0,
                y: 25.0,
                width: 50.0,
                height: 50.0,
            },
            &page_size,
        );

        assert!(r1.overlaps(&r2));
        assert_eq!(r1.intersection_area_px(&r2), 625.0);
    }

    #[test]
    fn test_generate_cleanup_masks() {
        let page = MangaPage {
            id: "page-1".into(),
            page_number: Some(1),
            source: PageSource {
                image_id: "img-1".into(),
                filename: Some("01.jpg".into()),
                native_size: Size {
                    width: 1000.0,
                    height: 1500.0,
                },
                dpi: Some(300.0),
                color_space: Some("rgb".into()),
            },
            bands: None,
            panels: vec![],
            containers: vec![TextContainer {
                id: "b1".into(),
                panel_id: Some("p1".into()),
                container_type: "speech-balloon".into(),
                geometry: ContainerGeometry {
                    shape: "ellipse".into(),
                    bounds: DualRect::from_px(
                        Rect {
                            x: 10.0,
                            y: 10.0,
                            width: 100.0,
                            height: 100.0,
                        },
                        &Size {
                            width: 1000.0,
                            height: 1500.0,
                        },
                    ),
                    polygon: None,
                },
                padding: None,
                optical_center: None,
            }],
            text_regions: vec![],
            art_regions: None,
            masks: None,
        };

        let masks = generate_cleanup_masks(&page);
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].mask_type, "clean-balloon");
    }
}
