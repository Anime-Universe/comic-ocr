//! Which filter removes what, and is any of it a real region?
//!
//! `where_regions_are_lost` showed three roughly equal loss channels at high
//! density: merged before filtering, removed by a filter, and found-but-under-
//! IoU. This opens the middle one. A filter removing panel frames is doing its
//! job; a filter removing balloons is a defect, and a single "after-filt" count
//! cannot tell them apart.
//!
//! Each dropped box is scored against the truth set: if it would have matched a
//! real region at IoU >= 0.5, removing it COST recall.
use comic_ocr_synth::detect::{Box2, DetectSpec};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use image::Luma;
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use imageproc::region_labelling::{Connectivity, connected_components};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn would_have_matched(b: &Box2, truth: &[Box2]) -> bool {
    truth.iter().any(|t| b.iou(t) >= 0.5)
}

fn main() {
    let font = SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    )
    .expect("font");
    let texts: Vec<String> = [
        "そうだね",
        "ちょっとまって",
        "ウソでしょ",
        "また迷路だし",
        "ぎゃっ",
        "少し黙っている",
        "実戦剣術も一流です",
        "素直にあやまるしか",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let spec = DetectSpec::default();
    println!(
        "{:>7}  {:>16} {:>16} {:>16} {:>16}",
        "truth", "min_area", "max_area_frac", "min_ink", "containers"
    );
    println!(
        "{:>7}  {:>16} {:>16} {:>16} {:>16}",
        "", "drop/cost", "drop/cost", "drop/cost", "drop/cost"
    );
    println!("{}", "-".repeat(80));

    for target in [8usize, 24, 60] {
        let mut rng = StdRng::seed_from_u64(4242);
        let page_spec = PageSpec {
            width: 1200,
            height: 1700,
            target_regions: target,
            ..Default::default()
        };
        let Ok((page, truth_page)) = render_page(&page_spec, &font, &texts, &mut rng) else {
            continue;
        };
        let truth: Vec<Box2> = truth_page
            .regions
            .iter()
            .map(|r| {
                let (x, y, w, h) = r.enclosure_bounds();
                Box2 {
                    x,
                    y,
                    width: w,
                    height: h,
                }
            })
            .collect();

        let level = otsu_level(&page);
        let binary = threshold(&page, level, ThresholdType::BinaryInverted);
        let merged = dilate(&binary, Norm::LInf, spec.merge_radius);
        let labels = connected_components(&merged, Connectivity::Eight, Luma([0u8]));
        let max_label = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);
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
        let r = spec.merge_radius as u32;
        let (mut d_min, mut c_min) = (0, 0);
        let (mut d_max, mut c_max) = (0, 0);
        let (mut d_ink, mut c_ink) = (0, 0);
        let mut kept = Vec::new();

        for (x0, y0, x1, y1) in acc.into_iter().flatten() {
            let b = Box2 {
                x: x0.saturating_add(r).min(x1),
                y: y0.saturating_add(r).min(y1),
                width: (x1.saturating_sub(x0)).saturating_sub(r * 2).max(1),
                height: (y1.saturating_sub(y0)).saturating_sub(r * 2).max(1),
            };
            if b.area() < spec.min_area {
                d_min += 1;
                if would_have_matched(&b, &truth) {
                    c_min += 1
                }
                continue;
            }
            if b.area() as f32 / page_area > spec.max_area_fraction {
                d_max += 1;
                if would_have_matched(&b, &truth) {
                    c_max += 1
                }
                continue;
            }
            let mut ink = 0u32;
            for yy in b.y..(b.y + b.height).min(page.height()) {
                for xx in b.x..(b.x + b.width).min(page.width()) {
                    if page.get_pixel(xx, yy).0[0] <= level {
                        ink += 1
                    }
                }
            }
            if (ink as f32 / b.area() as f32) < spec.min_ink_density {
                d_ink += 1;
                if would_have_matched(&b, &truth) {
                    c_ink += 1
                }
                continue;
            }
            kept.push(b);
        }

        let all = kept.clone();
        let mut d_con = 0;
        let mut c_con = 0;
        kept.retain(|b| {
            let contains_another = all.iter().any(|o| {
                o != b
                    && o.x >= b.x
                    && o.y >= b.y
                    && o.x + o.width <= b.x + b.width
                    && o.y + o.height <= b.y + b.height
            });
            if contains_another {
                d_con += 1;
                if would_have_matched(b, &truth) {
                    c_con += 1
                }
            }
            !contains_another
        });

        println!(
            "{:>7}  {:>16} {:>16} {:>16} {:>16}",
            truth.len(),
            format!("{d_min}/{c_min}"),
            format!("{d_max}/{c_max}"),
            format!("{d_ink}/{c_ink}"),
            format!("{d_con}/{c_con}"),
        );
    }
    println!();
    println!("drop = boxes the filter removed · cost = of those, how many would have");
    println!("matched a real region at IoU >= 0.50. cost > 0 means the filter is");
    println!("removing regions the detector had already found correctly.");
}
