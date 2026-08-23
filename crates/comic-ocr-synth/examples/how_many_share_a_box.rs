//! Is the recall loss one failure or several?
//!
//! `which_filter_drops` exonerated every filter — cost 0 at every density, so
//! nothing removed would have matched. That leaves merging, and this asks how
//! badly: how many TRUTH regions fall inside a single detected box.
//!
//! A box covering two balloons scores below IoU 0.50 against each of them, so
//! merging shows up twice — once as a missing detection and again as a
//! low-IoU one. Counting occupancy separates the two readings.
use comic_ocr_synth::detect::{Box2, DetectSpec, detect_regions, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Does `inner`'s centre fall inside `outer`?
fn centre_inside(inner: &Box2, outer: &Box2) -> bool {
    let cx = inner.x + inner.width / 2;
    let cy = inner.y + inner.height / 2;
    cx >= outer.x && cx < outer.x + outer.width && cy >= outer.y && cy < outer.y + outer.height
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
        "{:>7} {:>7} {:>8} {:>10} {:>12} {:>14}",
        "truth", "found", "matched", "unmatched", "in-a-shared", "worst-box"
    );
    println!("{}", "-".repeat(64));

    for target in [8usize, 16, 24, 40, 60] {
        let mut rng = StdRng::seed_from_u64(4242);
        let page_spec = PageSpec {
            width: 1200,
            height: 1700,
            target_regions: target,
            ..Default::default()
        };
        let Ok((page, tp)) = render_page(&page_spec, &font, &texts, &mut rng) else {
            continue;
        };
        let truth: Vec<Box2> = tp
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
        let found = detect_regions(&page, &spec);
        let rep = score(&truth, &found, 0.5);

        // How many truth regions sit inside each detected box.
        let mut worst = 0usize;
        let mut crowded = 0usize;
        for f in &found {
            let n = truth.iter().filter(|t| centre_inside(t, f)).count();
            if n > worst {
                worst = n
            }
            if n > 1 {
                crowded += n
            }
        }

        println!(
            "{:>7} {:>7} {:>8} {:>10} {:>12} {:>14}",
            truth.len(),
            found.len(),
            rep.matched,
            truth.len() - rep.matched,
            crowded,
            worst
        );
    }
    println!();
    println!("in-a-shared = truth regions sharing a detected box with another");
    println!("worst-box   = most truth regions inside one detected box");
    println!();
    println!("If in-a-shared accounts for most of unmatched, merging is THE failure,");
    println!("not one of several — and the fix is separation, not thresholds.");
}
