//! A free, local text-region detector, and the recall measurement it enables.
//!
//! `docs/GEOMETRY_PATH.md` records that `imageproc` already ships everything the
//! proposed front end needs — `contrast::otsu_level`, `morphology`,
//! `region_labelling::connected_components` — and that only `gaussian_blur_f32`
//! was ever imported. This is that front end.
//!
//! Why it matters beyond being cheap: **detection recall is unmeasured.** We
//! know how well text is read once found and nothing about what is never found.
//! Infinite-Verse#834 rests on two engines agreeing, which is agreement and not
//! correctness — two engines can agree and both be wrong. Against a page whose
//! regions are known by construction, recall becomes a number.
//!
//! This is deliberately NOT a good detector. It is a cheap, uncorrelated third
//! opinion whose failures differ from a vision model's, which is what makes a
//! disagreement informative.

use image::{GrayImage, Luma};
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use imageproc::region_labelling::{Connectivity, connected_components};

/// An axis-aligned box in page pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box2 {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Box2 {
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Intersection over union, the standard geometric agreement measure.
    pub fn iou(&self, other: &Box2) -> f32 {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width).min(other.x + other.width);
        let y1 = (self.y + self.height).min(other.y + other.height);
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let inter = ((x1 - x0) * (y1 - y0)) as f32;
        let union = self.area() as f32 + other.area() as f32 - inter;
        if union <= 0.0 { 0.0 } else { inter / union }
    }
}

#[derive(Debug, Clone)]
pub struct DetectSpec {
    /// Dilation radius used to merge glyphs into a block. Too small and every
    /// character is its own region; too large and neighbouring balloons merge.
    pub merge_radius: u8,
    /// Reject components smaller than this, in pixels.
    pub min_area: u32,
    /// Reject components larger than this fraction of the page -- a panel frame
    /// is a huge connected component and is not text.
    pub max_area_fraction: f32,
    /// Reject components whose bounding box is almost entirely empty.
    ///
    /// Measured 2026-08-22: this does NOT separate panel frames from balloons,
    /// because a balloon is also an outline with a sparse interior. Raising it
    /// to 0.08 to exclude frames took recall from 71.2% to 32.2% by discarding
    /// real balloons. It is kept only as a floor against near-empty noise;
    /// containers are removed by `drop_containers` instead.
    pub min_ink_density: f32,
    /// Drop any detection that fully contains another. A panel contains
    /// balloons; a balloon contains nothing. This is `GEOMETRY_PATH.md`'s
    /// containment-depth idea -- keep the leaves of the forest, not its roots.
    pub drop_containers: bool,
}

impl Default for DetectSpec {
    fn default() -> Self {
        Self {
            merge_radius: 6,
            min_area: 400,
            max_area_fraction: 0.20,
            min_ink_density: 0.01,
            drop_containers: true,
        }
    }
}

/// Find candidate text regions. No model, no network, no cost.
pub fn detect_regions(page: &GrayImage, spec: &DetectSpec) -> Vec<Box2> {
    let level = otsu_level(page);
    // Text is dark on light, so ThresholdBinaryInverted puts ink at 255 and the
    // component labeller counts ink rather than paper.
    let binary = threshold(page, level, ThresholdType::BinaryInverted);
    // Glyphs are separate components until merged; dilation closes the gaps
    // between characters of one block without reaching the next block.
    let merged = dilate(&binary, Norm::LInf, spec.merge_radius);

    let labels = connected_components(&merged, Connectivity::Eight, Luma([0u8]));
    let max_label = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);
    if max_label == 0 {
        return Vec::new();
    }

    // Accumulate a bounding box per label in one pass.
    let mut acc: Vec<Option<(u32, u32, u32, u32)>> = vec![None; max_label as usize + 1];
    for (x, y, px) in labels.enumerate_pixels() {
        let l = px.0[0] as usize;
        if l == 0 {
            continue;
        }
        match &mut acc[l] {
            Some((x0, y0, x1, y1)) => {
                *x0 = (*x0).min(x);
                *y0 = (*y0).min(y);
                *x1 = (*x1).max(x);
                *y1 = (*y1).max(y);
            }
            slot => *slot = Some((x, y, x, y)),
        }
    }

    let page_area = (page.width() * page.height()) as f32;
    let ink_level = level; // reuse Otsu's split: below it is ink
    let mut out = Vec::new();
    for entry in acc.into_iter().flatten() {
        let (x0, y0, x1, y1) = entry;
        // Undo the dilation so the box describes the ink, not the merge kernel.
        let r = spec.merge_radius as u32;
        let bx = x0.saturating_add(r).min(x1);
        let by = y0.saturating_add(r).min(y1);
        let bw = (x1.saturating_sub(x0)).saturating_sub(r * 2).max(1);
        let bh = (y1.saturating_sub(y0)).saturating_sub(r * 2).max(1);
        let b = Box2 {
            x: bx,
            y: by,
            width: bw,
            height: bh,
        };
        if b.area() < spec.min_area || b.area() as f32 / page_area > spec.max_area_fraction {
            continue;
        }
        // Measure ink on the ORIGINAL page, not the dilated mask -- dilation
        // inflates a thin outline into something that looks dense.
        let mut ink = 0u32;
        for yy in b.y..(b.y + b.height).min(page.height()) {
            for xx in b.x..(b.x + b.width).min(page.width()) {
                if page.get_pixel(xx, yy).0[0] <= ink_level {
                    ink += 1;
                }
            }
        }
        if (ink as f32 / b.area() as f32) < spec.min_ink_density {
            continue;
        }
        out.push(b);
    }
    if spec.drop_containers {
        let all = out.clone();
        out.retain(|b| {
            !all.iter().any(|o| {
                o != b
                    && o.x >= b.x
                    && o.y >= b.y
                    && o.x + o.width <= b.x + b.width
                    && o.y + o.height <= b.y + b.height
            })
        });
    }

    out.sort_by_key(|b| (b.y, b.x));
    out
}

/// Recall, precision and their matching, at an IoU threshold.
#[derive(Debug, Clone)]
pub struct RecallReport {
    pub truth_count: usize,
    pub detected_count: usize,
    pub matched: usize,
    pub iou_threshold: f32,
    /// Fraction of true regions found. The number nothing currently measures.
    pub recall: f32,
    /// Fraction of detections that correspond to a true region.
    pub precision: f32,
    /// Mean IoU over matched pairs -- how well the boxes agree, not just that
    /// they overlap.
    pub mean_matched_iou: f32,
}

/// Greedy one-to-one matching, best IoU first. A truth region may be claimed
/// only once, so splitting one region into three detections scores one match
/// and two false positives rather than three matches.
pub fn score(truth: &[Box2], detected: &[Box2], iou_threshold: f32) -> RecallReport {
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for (ti, t) in truth.iter().enumerate() {
        for (di, d) in detected.iter().enumerate() {
            let v = t.iou(d);
            if v >= iou_threshold {
                pairs.push((v, ti, di));
            }
        }
    }
    pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    let mut used_t = vec![false; truth.len()];
    let mut used_d = vec![false; detected.len()];
    let (mut matched, mut iou_sum) = (0usize, 0.0f32);
    for (v, ti, di) in pairs {
        if used_t[ti] || used_d[di] {
            continue;
        }
        used_t[ti] = true;
        used_d[di] = true;
        matched += 1;
        iou_sum += v;
    }

    RecallReport {
        truth_count: truth.len(),
        detected_count: detected.len(),
        matched,
        iou_threshold,
        recall: if truth.is_empty() {
            0.0
        } else {
            matched as f32 / truth.len() as f32
        },
        precision: if detected.is_empty() {
            0.0
        } else {
            matched as f32 / detected.len() as f32
        },
        mean_matched_iou: if matched == 0 {
            0.0
        } else {
            iou_sum / matched as f32
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iou_is_zero_when_disjoint_and_one_when_identical() {
        let a = Box2 {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let b = Box2 {
            x: 100,
            y: 100,
            width: 10,
            height: 10,
        };
        assert_eq!(a.iou(&b), 0.0);
        assert_eq!(a.iou(&a), 1.0);
    }

    #[test]
    fn iou_halves_on_half_overlap() {
        let a = Box2 {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let b = Box2 {
            x: 5,
            y: 0,
            width: 10,
            height: 10,
        };
        // intersection 50, union 150
        assert!((a.iou(&b) - (50.0 / 150.0)).abs() < 1e-5);
    }

    #[test]
    fn splitting_one_region_costs_precision_not_recall() {
        let truth = vec![Box2 {
            x: 0,
            y: 0,
            width: 100,
            height: 20,
        }];
        let split = vec![
            Box2 {
                x: 0,
                y: 0,
                width: 100,
                height: 20,
            },
            Box2 {
                x: 0,
                y: 0,
                width: 90,
                height: 20,
            },
            Box2 {
                x: 5,
                y: 0,
                width: 95,
                height: 20,
            },
        ];
        let r = score(&truth, &split, 0.5);
        assert_eq!(r.matched, 1, "a truth region must be claimed only once");
        assert_eq!(r.recall, 1.0);
        assert!(r.precision < 0.4, "two spurious boxes must cost precision");
    }

    #[test]
    fn a_missed_region_shows_up_as_recall_not_precision() {
        let truth = vec![
            Box2 {
                x: 0,
                y: 0,
                width: 50,
                height: 50,
            },
            Box2 {
                x: 200,
                y: 200,
                width: 50,
                height: 50,
            },
        ];
        let found = vec![Box2 {
            x: 0,
            y: 0,
            width: 50,
            height: 50,
        }];
        let r = score(&truth, &found, 0.5);
        assert_eq!(r.recall, 0.5);
        assert_eq!(r.precision, 1.0);
    }

    #[test]
    fn a_hollow_frame_is_rejected_and_a_filled_block_is_kept() {
        let mut page = GrayImage::from_pixel(600, 600, Luma([250]));
        // A hollow rectangle -- a panel frame.
        for x in 40..560 {
            page.put_pixel(x, 40, Luma([20]));
            page.put_pixel(x, 400, Luma([20]));
        }
        for y in 40..400 {
            page.put_pixel(40, y, Luma([20]));
            page.put_pixel(559, y, Luma([20]));
        }
        // A solid block inside it -- text.
        for y in 200..250 {
            for x in 200..300 {
                page.put_pixel(x, y, Luma([20]));
            }
        }
        let found = detect_regions(&page, &DetectSpec::default());
        assert_eq!(
            found.len(),
            1,
            "frame must be rejected, block kept: {found:?}"
        );
        assert!(
            found[0].width < 200,
            "kept the frame instead of the block: {:?}",
            found[0]
        );
    }

    #[test]
    fn detector_finds_a_drawn_block_and_ignores_an_empty_page() {
        let blank = GrayImage::from_pixel(400, 400, Luma([250]));
        assert!(detect_regions(&blank, &DetectSpec::default()).is_empty());

        let mut page = GrayImage::from_pixel(400, 400, Luma([250]));
        for y in 100..160 {
            for x in 100..220 {
                page.put_pixel(x, y, Luma([20]));
            }
        }
        let found = detect_regions(&page, &DetectSpec::default());
        assert_eq!(
            found.len(),
            1,
            "one block should give one region, got {found:?}"
        );
    }
}
