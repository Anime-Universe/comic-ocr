//! Where does recall go on a dense page?
//!
//! `measure_recall` shows the number falling with density — 87.5% at 8 regions,
//! 57.8% at 60 — and a radius sweep showed r=6 is optimal at EVERY density, so
//! the merge kernel is not what changes. This asks what does.
//!
//! Counts components at each stage of `detect_regions`, so a lost region is
//! attributed rather than inferred.
use comic_ocr_synth::detect::{Box2, DetectSpec, detect_regions, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use image::Luma;
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use imageproc::region_labelling::{Connectivity, connected_components};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
        "{:>7} {:>6} {:>10} {:>9} {:>8} {:>8}",
        "truth", "comps", "after-filt", "recall", "matched", "lost"
    );
    println!("{}", "-".repeat(56));

    for target in [8usize, 16, 24, 40, 60] {
        let mut rng = StdRng::seed_from_u64(4242);
        let page_spec = PageSpec {
            width: 1200,
            height: 1700,
            target_regions: target,
            ..Default::default()
        };
        let Ok((page, truth)) = render_page(&page_spec, &font, &texts, &mut rng) else {
            continue;
        };

        // Stage 1: raw connected components after the same dilation.
        let level = otsu_level(&page);
        let binary = threshold(&page, level, ThresholdType::BinaryInverted);
        let merged = dilate(&binary, Norm::LInf, spec.merge_radius);
        let labels = connected_components(&merged, Connectivity::Eight, Luma([0u8]));
        let comps = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);

        // Stage 2: what survives every filter.
        let found = detect_regions(&page, &spec);

        let boxes: Vec<Box2> = truth
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
        let rep = score(&boxes, &found, 0.5);

        println!(
            "{:>7} {:>6} {:>10} {:>8.1}% {:>8} {:>8}",
            boxes.len(),
            comps,
            found.len(),
            100.0 * rep.recall,
            rep.matched,
            boxes.len() - rep.matched
        );
    }
    println!();
    println!("comps  = connected components after dilation, before any filter");
    println!("If comps >= truth but after-filt < truth, regions are being FILTERED away.");
    println!("If comps < truth, they were MERGED before any filter could see them.");
}
