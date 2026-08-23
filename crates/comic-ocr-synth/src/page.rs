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
use imageproc::drawing::{draw_filled_ellipse_mut, draw_hollow_ellipse_mut, draw_hollow_rect_mut};
use imageproc::rect::Rect;
use rand::Rng;

/// What encloses a region. A detector trained on comics keys on these shapes,
/// so a page of bare text blocks cannot ground-truth one -- it is a word cloud,
/// not a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Enclosure {
    /// Rounded speech balloon with a white interior and an ink outline.
    Balloon,
    /// Rectangular narration/caption box.
    CaptionBox,
    /// Nothing drawn -- free text over the page, as sound effects and signage sit.
    None,
}

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
    pub enclosure: Enclosure,
    /// Index into `PageTruth::panels`, when the region falls inside one.
    pub panel: Option<usize>,
}

impl RegionTruth {
    /// The footprint the enclosure actually occupies, which is larger than the
    /// text box for a balloon and slightly larger for a caption box.
    pub fn enclosure_bounds(&self) -> (u32, u32, u32, u32) {
        match self.enclosure {
            Enclosure::Balloon => {
                let mx = ((self.width as f32 * 0.71).ceil() as u32).saturating_sub(self.width / 2);
                let my =
                    ((self.height as f32 * 0.71).ceil() as u32).saturating_sub(self.height / 2);
                (
                    self.x.saturating_sub(mx),
                    self.y.saturating_sub(my),
                    self.width + mx * 2,
                    self.height + my * 2,
                )
            }
            Enclosure::CaptionBox => (
                self.x.saturating_sub(5),
                self.y.saturating_sub(5),
                self.width + 10,
                self.height + 10,
            ),
            Enclosure::None => (self.x, self.y, self.width, self.height),
        }
    }
}

/// A drawn panel frame. Regions are attributed to whichever panel contains them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PanelTruth {
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageTruth {
    pub width: u32,
    pub height: u32,
    /// What was actually drawn. The detector's count is compared against this.
    pub region_count: usize,
    pub regions: Vec<RegionTruth>,
    pub panels: Vec<PanelTruth>,
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
    /// Divide the page into panel frames before placing text. Off produces the
    /// bare-blocks page, which is adequate for counting and useless for
    /// ground-truthing a detector.
    pub draw_panels: bool,
    /// Rows x columns of panels when `draw_panels` is set.
    pub panel_grid: (u32, u32),
    /// Lay panels out by guillotine subdivision rather than a uniform grid.
    /// A grid produces identical panels at identical offsets on every page,
    /// which a detector can exploit without generalising.
    pub irregular_panels: bool,
    /// How many panels to subdivide into when `irregular_panels` is set.
    pub panel_count: u32,
}

impl Default for PageSpec {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 1700,
            target_regions: 20,
            ground: 250,
            draw_borders: true,
            draw_panels: true,
            panel_grid: (4, 2),
            irregular_panels: false,
            panel_count: 6,
        }
    }
}

/// `a` is a candidate's enclosure footprint; `b` is an already-placed region,
/// compared by ITS enclosure footprint rather than its text box.
fn overlaps(a: (u32, u32, u32, u32), b: &RegionTruth, pad: u32) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b.enclosure_bounds();
    ax < bx + bw + pad && bx < ax + aw + pad && ay < by + bh + pad && by < ay + ah + pad
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
    let ink = Luma([40u8]);

    // Panels first: text sits inside them, so they must exist before placement.
    let mut panels: Vec<PanelTruth> = Vec::new();
    if spec.draw_panels && spec.irregular_panels {
        // A UNIFORM GRID IS NOT A MANGA PAGE. The fixed 4x2 layout put eight
        // identical panels at identical offsets on every page, and a detector
        // measured against it inherits that regularity: the empty-frame failure
        // measured at 13-of-13 boxes of exactly 575x400 turned out not to occur
        // on real pages at all, because real pages have no such lattice.
        //
        // Guillotine subdivision instead: split the page recursively, always
        // cutting the largest remaining rectangle, at a ratio drawn away from
        // the midpoint. That produces irregular panels of varying size and
        // aspect -- structurally what a manga layout is, since most are built
        // from full-width or full-height cuts rather than from a grid.
        let gutter = 18u32;
        let margin = 12u32;
        let mut rects: Vec<(u32, u32, u32, u32)> = vec![(
            margin,
            margin,
            spec.width.saturating_sub(margin * 2),
            spec.height.saturating_sub(margin * 2),
        )];
        let want = spec.panel_count.max(1) as usize;
        while rects.len() < want {
            // Always split the largest, so panels stay comparable in scale
            // rather than degenerating into one huge rect and many slivers.
            let (bi, _) = rects
                .iter()
                .enumerate()
                .max_by_key(|(_, r)| r.2 as u64 * r.3 as u64)
                .unwrap();
            let (x, y, w, h) = rects.swap_remove(bi);
            // Cut across the longer axis; ratio away from centre so panels
            // differ in size, which a grid never produces.
            let ratio = rng.gen_range(0.32f32..0.68f32);
            let (a, b) = if w >= h {
                let cut = (w as f32 * ratio) as u32;
                ((x, y, cut, h), (x + cut, y, w - cut, h))
            } else {
                let cut = (h as f32 * ratio) as u32;
                ((x, y, w, cut), (x, y + cut, w, h - cut))
            };
            if a.2 < 80 || a.3 < 80 || b.2 < 80 || b.3 < 80 {
                rects.push((x, y, w, h));
                break;
            }
            rects.push(a);
            rects.push(b);
        }
        for (x, y, w, h) in rects {
            let (px, py) = (x + gutter / 2, y + gutter / 2);
            let (pw, ph) = (w.saturating_sub(gutter), h.saturating_sub(gutter));
            if pw < 40 || ph < 40 {
                continue;
            }
            draw_hollow_rect_mut(&mut page, Rect::at(px as i32, py as i32).of_size(pw, ph), ink);
            panels.push(PanelTruth {
                index: panels.len(),
                x: px,
                y: py,
                width: pw,
                height: ph,
            });
        }
    } else if spec.draw_panels {
        let (rows, cols) = spec.panel_grid;
        let (rows, cols) = (rows.max(1), cols.max(1));
        let gutter = 18u32;
        let cell_w = spec.width / cols;
        let cell_h = spec.height / rows;
        for r in 0..rows {
            for c in 0..cols {
                let x = c * cell_w + gutter;
                let y = r * cell_h + gutter;
                let w = cell_w.saturating_sub(gutter * 2);
                let h = cell_h.saturating_sub(gutter * 2);
                if w < 40 || h < 40 {
                    continue;
                }
                draw_hollow_rect_mut(&mut page, Rect::at(x as i32, y as i32).of_size(w, h), ink);
                panels.push(PanelTruth {
                    index: panels.len(),
                    x,
                    y,
                    width: w,
                    height: h,
                });
            }
        }
    }

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

        let enclosure = if !spec.draw_borders {
            Enclosure::None
        } else if rng.gen_bool(0.75) {
            Enclosure::Balloon
        } else if rng.gen_bool(0.6) {
            Enclosure::CaptionBox
        } else {
            Enclosure::None
        };

        // A balloon's ellipse extends well past the text box it encloses, so
        // overlap and panel fit must both use the ENCLOSURE footprint. Checking
        // text boxes let balloons collide and produced unreadable overlaps.
        let (mx, my) = match enclosure {
            Enclosure::Balloon => (
                ((bw as f32 * 0.71).ceil() as u32).saturating_sub(bw / 2),
                ((bh as f32 * 0.71).ceil() as u32).saturating_sub(bh / 2),
            ),
            Enclosure::CaptionBox => (5, 5),
            Enclosure::None => (0, 0),
        };
        let (fw, fh) = (bw + mx * 2, bh + my * 2);

        // Inside a panel when there is one: real balloons sit within their
        // frame far more often than across it.
        let bounds = if panels.is_empty() {
            (
                4u32,
                4u32,
                spec.width.saturating_sub(4),
                spec.height.saturating_sub(4),
            )
        } else {
            let p = &panels[rng.gen_range(0..panels.len())];
            (p.x + 2, p.y + 2, p.x + p.width - 2, p.y + p.height - 2)
        };
        if bounds.2 <= bounds.0 + fw || bounds.3 <= bounds.1 + fh {
            continue;
        }
        let fx = rng.gen_range(bounds.0..(bounds.2 - fw));
        let fy = rng.gen_range(bounds.1..(bounds.3 - fh));
        let (x, y) = (fx + mx, fy + my);
        if placed.iter().any(|p| overlaps((fx, fy, fw, fh), p, 6)) {
            continue;
        }

        match enclosure {
            Enclosure::Balloon => {
                // Ellipse must clear the text box's corners, hence the sqrt(2).
                let cx = (x + bw / 2) as i32;
                let cy = (y + bh / 2) as i32;
                let rx = ((bw as f32 * 0.71).ceil() as i32).max(6);
                let ry = ((bh as f32 * 0.71).ceil() as i32).max(6);
                draw_filled_ellipse_mut(&mut page, (cx, cy), rx, ry, Luma([spec.ground]));
                draw_hollow_ellipse_mut(&mut page, (cx, cy), rx, ry, ink);
            }
            Enclosure::CaptionBox => {
                let pad = 5u32;
                let bx = x.saturating_sub(pad);
                let by = y.saturating_sub(pad);
                let bwid = (bw + pad * 2).min(spec.width - bx);
                let bhei = (bh + pad * 2).min(spec.height - by);
                draw_hollow_rect_mut(
                    &mut page,
                    Rect::at(bx as i32, by as i32).of_size(bwid, bhei),
                    ink,
                );
            }
            Enclosure::None => {}
        }
        image::imageops::overlay(&mut page, &block, x as i64, y as i64);

        let panel = panels.iter().position(|p| {
            x >= p.x && y >= p.y && x + bw <= p.x + p.width && y + bh <= p.y + p.height
        });

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
            enclosure,
            panel,
        });
    }

    let truth = PageTruth {
        width: spec.width,
        height: spec.height,
        region_count: placed.len(),
        regions: placed,
        panels,
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
    fn panels_are_recorded_and_regions_attributed_to_them() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(11);
        let spec = PageSpec {
            target_regions: 24,
            panel_grid: (4, 2),
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        assert_eq!(truth.panels.len(), 8, "4x2 grid should record 8 panels");
        // Attribution must be real containment, not an index handed out blindly.
        for r in truth.regions.iter().filter(|r| r.panel.is_some()) {
            let p = &truth.panels[r.panel.unwrap()];
            assert!(
                r.x >= p.x
                    && r.y >= p.y
                    && r.x + r.width <= p.x + p.width
                    && r.y + r.height <= p.y + p.height,
                "region {} claims panel {} but is not inside it",
                r.index,
                p.index
            );
        }
    }

    #[test]
    fn enclosures_are_drawn_and_recorded() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(12);
        let spec = PageSpec {
            target_regions: 30,
            ..Default::default()
        };
        let (page, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        let balloons = truth
            .regions
            .iter()
            .filter(|r| r.enclosure == Enclosure::Balloon)
            .count();
        assert!(balloons > 0, "expected some balloons at p=0.75");

        // A balloon must actually put ink outside its text box, or the record
        // claims an enclosure the pixels do not have.
        let b = truth
            .regions
            .iter()
            .find(|r| r.enclosure == Enclosure::Balloon)
            .unwrap();
        // The ellipse clears the text box's corners, so its right edge sits at
        // cx + 0.71*bw -- about x + 1.21*bw, not just past the box. Probing at
        // the box edge found white and failed a correct drawing.
        let cy = b.y + b.height / 2;
        let cx = b.x + b.width / 2;
        let rx = (b.width as f32 * 0.71).ceil() as u32;
        let edge = (cx + rx).min(page.width() - 1);
        let ring: u32 = (0..6)
            .filter_map(|d| {
                let x = edge.saturating_sub(d);
                (page.get_pixel(x, cy).0[0] < 160).then_some(1u32)
            })
            .sum();
        assert!(
            ring > 0,
            "no balloon outline at the ellipse edge (x~{edge}, y={cy}) for region {}",
            b.index
        );
    }

    #[test]
    fn without_panels_none_are_recorded() {
        let Some(f) = font() else { return };
        let mut rng = StdRng::seed_from_u64(13);
        let spec = PageSpec {
            draw_panels: false,
            target_regions: 6,
            ..Default::default()
        };
        let (_, truth) = render_page(&spec, &f, &texts(), &mut rng).expect("page");
        assert!(truth.panels.is_empty());
        assert!(truth.regions.iter().all(|r| r.panel.is_none()));
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
