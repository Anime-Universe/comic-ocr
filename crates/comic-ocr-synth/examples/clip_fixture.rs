//! Pages with a known region count, for validating detector clipping.
//!
//! Specified by `infinite-verse-c3` for manga-service#117 (Infinite-Verse#859).
//! It is a COUNTING question — region content is irrelevant and may repeat — so
//! nothing about transcription quality or layout geometry contaminates it.
//!
//! The prediction #117 makes, quoted so this fixture can falsify it:
//!
//!   pages under 120  -> regions_discarded = 0, kept == rendered
//!   pages over 120   -> exactly 120 kept, rendered-minus-120 discarded
//!
//! If reported-kept ever lands on 120 with discarded = 0, #117 is wrong.
//! That is the case this fixture exists to expose, before it merges.
//!
//!   cargo run -p comic-ocr-synth --example clip_fixture -- <out-dir>
//!
//! Writes page-NNN.png alongside page-NNN.json, and manifest.json listing every
//! page with the count actually rendered.

use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/clip-fixture".into());
    let font_path = std::env::var("COMIC_OCR_SYNTH_FONT")
        .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into());
    let Ok(font) = SynthFont::from_path(&font_path, 0) else {
        eprintln!("no font at {font_path}");
        std::process::exit(2);
    };
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("{out}: {e}");
        std::process::exit(1);
    }

    let texts: Vec<String> = [
        "そうだね",
        "ちょっとまって",
        "ウソでしょ",
        "また迷路だし",
        "ぎゃっ",
        "少し黙っている",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut manifest = Vec::new();
    println!("{:>8} {:>10} {:>10}   file", "target", "rendered", "expect");
    println!("{}", "-".repeat(58));

    for target in (100..=200).step_by(10) {
        // Seeded per page so the whole fixture regenerates identically.
        let mut rng = StdRng::seed_from_u64(20260822 + target as u64);
        let spec = PageSpec {
            width: 2400,
            height: 3400,
            target_regions: target,
            draw_borders: false, // borders cost space and this is a counting test
            draw_panels: false,  // ditto: panel frames only reduce placeable area
            panel_grid: (1, 1),
            ground: 250,
        };
        let Ok((img, truth)) = render_page(&spec, &font, &texts, &mut rng) else {
            eprintln!("target {target}: render failed");
            continue;
        };
        let stem = format!("page-{target:03}");
        if img.save(format!("{out}/{stem}.png")).is_err() {
            eprintln!("{stem}: save failed");
            continue;
        }
        let json = serde_json::to_string_pretty(&truth).expect("serialise");
        let _ = std::fs::write(format!("{out}/{stem}.json"), &json);

        // What #117 should report for this page, stated up front so a run can be
        // checked without re-deriving the rule.
        let rendered = truth.region_count;
        let (kept, discarded) = if rendered > 120 {
            (120, rendered - 120)
        } else {
            (rendered, 0)
        };
        println!(
            "{target:>8} {rendered:>10} {:>10}   {stem}.png",
            format!("{kept}/{discarded}")
        );

        manifest.push(serde_json::json!({
            "file": format!("{stem}.png"),
            "truth": format!("{stem}.json"),
            "target_regions": target,
            "rendered_regions": rendered,
            "expected_kept": kept,
            "expected_discarded": discarded,
        }));
    }

    let m = serde_json::json!({
        "purpose": "validate detector clipping for manga-service#117 / Infinite-Verse#859",
        "cap": 120,
        "note": "rendered_regions is what was actually drawn, never what was requested",
        "pages": manifest,
    });
    let _ = std::fs::write(
        format!("{out}/manifest.json"),
        serde_json::to_string_pretty(&m).expect("serialise"),
    );
    println!(
        "\nwrote {} pages + manifest.json to {out}",
        m["pages"].as_array().map_or(0, Vec::len)
    );
    println!(
        "\nFalsifies #117 if any page reports kept == 120 with discarded == 0,\n\
         or if a page under the cap reports discarded > 0."
    );
}
