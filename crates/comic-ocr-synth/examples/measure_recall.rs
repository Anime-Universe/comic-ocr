//! Detection recall against pages whose regions are known by construction.
//!
//! This is the number `docs/GEOMETRY_PATH.md` calls the unmeasured risk: we know
//! how well text is read once found, and nothing about what is never found.
//! Infinite-Verse#834 rests on two engines agreeing across a density range,
//! which is agreement rather than correctness -- and #834's own caveat says
//! neither engine is known-correct until #833 exists.
//!
//! Free to run: no model, no network, no API spend.
//!
//!   cargo run -p comic-ocr-synth --example measure_recall
//!
//! The detector here is deliberately cheap. A poor recall number is a fact
//! about THIS detector, not about the platform's -- what transfers is the
//! method and the fixture.

use comic_ocr_synth::detect::{Box2, DetectSpec, detect_regions, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    let font = match SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    ) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
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

    println!(
        "{:>7} {:>6} {:>6} {:>6} {:>8} {:>9} {:>9}",
        "regions", "truth", "found", "match", "recall", "precision", "mean IoU"
    );
    println!("{}", "-".repeat(60));

    let spec_detect = DetectSpec::default();
    let (mut r_sum, mut p_sum, mut n) = (0.0f32, 0.0f32, 0usize);

    for target in [8usize, 16, 24, 40, 60] {
        let mut rng = StdRng::seed_from_u64(20260822 + target as u64);
        let page_spec = PageSpec {
            width: 1200,
            height: 1700,
            target_regions: target,
            ..Default::default()
        };
        let Ok((page, truth)) = render_page(&page_spec, &font, &texts, &mut rng) else {
            continue;
        };
        // Score against the ENCLOSURE, since that is the visible region a
        // detector can see. Scoring against the text box would penalise a
        // correct balloon detection for including its own outline.
        let truth_boxes: Vec<Box2> = truth
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

        let found = detect_regions(&page, &spec_detect);
        let rep = score(&truth_boxes, &found, 0.5);
        println!(
            "{target:>7} {:>6} {:>6} {:>6} {:>7.1}% {:>8.1}% {:>9.3}",
            rep.truth_count,
            rep.detected_count,
            rep.matched,
            100.0 * rep.recall,
            100.0 * rep.precision,
            rep.mean_matched_iou
        );
        r_sum += rep.recall;
        p_sum += rep.precision;
        n += 1;
    }

    if n > 0 {
        println!("{}", "-".repeat(60));
        println!(
            "mean over {n} pages   recall {:.1}%   precision {:.1}%   (IoU >= 0.50)",
            100.0 * r_sum / n as f32,
            100.0 * p_sum / n as f32
        );
    }
    println!(
        "\nRecall is the half nobody measures. A page whose text is never boxed\n\
         contributes nothing to the corpus and leaves no trace that it did not."
    );
}
