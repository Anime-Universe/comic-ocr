//! Compose text regions onto a page, with ground truth for every region.
//!
//! Two things need this, and they turn out to be the same thing.
//!
//! A page fixture with a KNOWN region count validates the detector's clipping
//! behaviour (Infinite-Verse#859 / manga-service#117) — a counting question,
//! independent of transcription quality.
//!
//! And `examples/compare_confidence` measured synthetic crops as uniformly
//! easier than real ones, with scan degradation closing only a fifth of the gap.
//! What is left is typography, ground, and **crop context**: real boxes clip
//! balloon borders, tails, and slivers of neighbouring text. A crop taken from a
//! composed page carries that context; a crop rendered in isolation cannot.

use crate::render::{Direction, RenderSpec, SynthFont, render};
use image::{GrayImage, Luma};
use rand::Rng;

/// Where a region landed, in page pixels, with the text that is truly there.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionTruth {
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub direction: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageTruth {
    pub width: u32,
    pub height: u32,
    /// What was actually drawn. The detector's count is compared against this.
    pub region_count: usize,
    pub regions: Vec<RegionTruth>,
}

pub struct PageSpec {
    pub width: u32,
    pub height: u32,
    /// How many regions to place. Placement stops early if the page fills up,
    /// and `PageTruth::region_count` reports what was actually drawn -- never
    /// what was requested, or the ground truth would be a claim rather than a
    /// record.
    pub target_regions: usize,
    pub ground: u8,
    /// Draw a balloon outline around each region, so crops taken from the page
    /// clip a border the way real ones do.
    pub draw_borders: bool,
}

impl Default for PageSpec {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 1700,
            target_regions: 20,
            ground: 250,
            draw_borders: true,
        }
    }
}

fn overlaps(a: (u32, u32, u32, u32), b: &RegionTruth, pad: u32) -> bool {
    let (ax, ay, aw, ah) = a;
    ax < b.x + b.width + pad
        && b.x < ax + aw + pad
        && ay < b.y + b.height + pad
        && b.y < ay + ah + pad
}

/// Place `target_regions` non-overlapping text blocks and report exactly what
/// landed.
pub fn render_page<R: Rng>(
    spec: &PageSpec,
    font: &SynthFont,
    texts: &[String],
    rng: &mut R,
) -> Result<(GrayImage, PageTruth), String> {
    if texts.is_empty() {
        return Err("need at least one text to place".into());
    }
    let mut page = GrayImage::from_pixel(spec.width, spec.height, Luma([spec.ground]));
    let mut placed: Vec<RegionTruth> = Vec::new();

    // Bounded attempts: a page that cannot fit the target must stop, not spin.
    let max_attempts = spec.target_regions * 40;
    let mut attempts = 0;

    while placed.len() < spec.target_regions && attempts < max_attempts {
        attempts += 1;
        let text = &texts[rng.gen_range(0..texts.len())];
        let chars = text.chars().filter(|c| !c.is_whitespace()).count().max(1);
        let vertical = rng.gen_bool(0.75); // manga balloons are mostly vertical
        let direction = if vertical {
            Direction::VerticalRl
        } else {
            Direction::HorizontalTb
        };
        let per_run = rng.gen_range(3..=chars.max(3)).min(chars);
        let font_px = rng.gen_range(16.0..=30.0);

        let rspec = RenderSpec {
            text: text.clone(),
            direction,
            font_px,
            cells_per_run: per_run,
            padding_px: 6,
            ground: spec.ground,
            ..Default::default()
        };
        let Ok(block) = render(&rspec, font) else {
            continue;
        };
        let (bw, bh) = (block.width(), block.height());
        if bw + 8 >= spec.width || bh + 8 >= spec.height {
            continue;
        }

        let x = rng.gen_range(4..(spec.width - bw - 4));
        let y = rng.gen_range(4..(spec.height - bh - 4));
        if placed.iter().any(|p| overlaps((x, y, bw, bh), p, 6)) {
            continue;
        }

        image::imageops::overlay(&mut page, &block, x as i64, y as i64);
        if spec.draw_borders {
            let ink = Luma([90u8]);
            for dx in 0..bw {
                page.put_pixel(x + dx, y, ink);
                page.put_pixel(x + dx, y + bh - 1, ink);
            }
            for dy in 0..bh {
                page.put_pixel(x, y + dy, ink);
                page.put_pixel(x + bw - 1, y + dy, ink);
            }
        }

        placed.push(RegionTruth {
            index: placed.len(),
            x,
            y,
            width: bw,
            height: bh,
            text: text.clone(),
            direction: if vertical {
                "vertical-rl".into()
            } else {
                "horizontal-tb".into()
            },
        });
    }

    let truth = PageTruth {
        width: spec.width,
        height: spec.height,
        region_count: placed.len(),
        regions: placed,
    };
    Ok((page, truth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn font() -> Option<SynthFont> {
        SynthFont::from_path("/System/Library/Fonts/Hiragino Sans GB.ttc", 0).ok()
    }

    fn texts() -> Vec<String> {
        ["そうだね", "ちょっとまって", "ウソでしょ", "また迷路だし"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn truth_reports_what_landed_not_what_was_asked_for() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(1);
        let spec = PageSpec {
            target_regions: 12,
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        assert_eq!(truth.region_count, truth.regions.len());
        assert!(truth.region_count <= 12);
    }

    #[test]
    fn regions_do_not_overlap() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(2);
        let spec = PageSpec {
            target_regions: 30,
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        for (i, a) in truth.regions.iter().enumerate() {
            for b in truth.regions.iter().skip(i + 1) {
                let hit = a.x < b.x + b.width
                    && b.x < a.x + a.width
                    && a.y < b.y + b.height
                    && b.y < a.y + a.height;
                assert!(!hit, "regions {} and {} overlap", a.index, b.index);
            }
        }
    }

    #[test]
    fn a_page_too_small_stops_instead_of_spinning() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(3);
        let spec = PageSpec {
            width: 200,
            height: 200,
            target_regions: 500,
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        assert!(
            truth.region_count < 500,
            "must not claim to place what did not fit"
        );
    }

    #[test]
    fn a_dense_page_can_exceed_the_detector_cap() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(4);
        let spec = PageSpec {
            width: 2400,
            height: 3400,
            target_regions: 160,
            draw_borders: false,
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        // manga-service caps at DETECTION_MAX_REGIONS = 120; the fixture is
        // useless unless it can get past that.
        assert!(
            truth.region_count > 120,
            "only placed {}",
            truth.region_count
        );
    }

    #[test]
    fn same_seed_gives_the_same_page() {
        let Some(f) = font() else { return };
        let build = || {
            let mut rng = StdRng::seed_from_u64(9);
            render_page(&PageSpec::default(), &f, &texts(), &mut rng).unwrap()
        };
        let (a, ta) = build();
        let (b, tb) = build();
        assert_eq!(a.into_raw(), b.into_raw());
        assert_eq!(ta.region_count, tb.region_count);
    }
}
