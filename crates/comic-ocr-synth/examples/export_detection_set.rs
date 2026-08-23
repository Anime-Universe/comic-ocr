//! Emit a detection training set: pages, and every region on each, known by
//! construction.
//!
//! Detection is the foundation. A transcriber cannot read a balloon nothing
//! located, and a scene graph cannot relate panels nothing bounded — so the
//! finder has to be right before anything downstream can be.
//!
//! The thing that makes this worth running today: **these labels need no human**.
//! The generator placed every region, so the truth is exact and unlimited. The
//! reader's training waits on an attested corpus; the finder's does not.
//!
//!   cargo run -p comic-ocr-synth --example export_detection_set -- out/ 40
//!
//! Writes `page-NNNN.png` beside `page-NNNN.json` conforming to
//! `schemas/detection_sample.json`, and a `summary.json` recording what the set
//! contains — because a training set whose composition nobody stated is a set
//! nobody can reason about when the model underperforms on some slice of it.
use comic_ocr_synth::degrade::{DegradeSpec, ScanQuality, apply as degrade_apply};
use comic_ocr_synth::page::{Enclosure, PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::json;

/// Lines with a range of lengths and scripts, so region shapes vary the way
/// they do on a real page. A set whose every region is the same size teaches a
/// detector the size rather than the thing.
const TEXTS: &[&str] = &[
    "そうだね",
    "ちょっとまって",
    "ウソでしょ",
    "また迷路だし",
    "ぎゃっ",
    "少し黙っている",
    "実戦剣術も一流です",
    "素直にあやまるしか",
    "きのうハンパーヶとって",
    "ピンポーーン",
    "第30話重苦しい闇の奥",
    "なるほど",
];

/// The generator names writing modes the way CSS does; the iPub envelope names
/// text direction its own way. Mapping rather than passing through, because a
/// label that does not match its own schema is a wrong value, and a wrong value
/// is worse than a missing one — it answers.
///
///   vertical-rl   vertical text, columns stacking right-to-left  -> ttb
///   horizontal-tb horizontal text, lines stacking top-to-bottom  -> ltr
///
/// Anything else is refused rather than guessed. If the generator learns a new
/// writing mode, this must be taught it deliberately.
fn direction_name(direction: &str) -> Option<&'static str> {
    match direction {
        "vertical-rl" => Some("ttb"),
        "horizontal-tb" => Some("ltr"),
        _ => None,
    }
}

fn enclosure_name(enclosure: &Enclosure) -> &'static str {
    match enclosure {
        Enclosure::Balloon => "balloon",
        Enclosure::CaptionBox => "caption-box",
        Enclosure::None => "none",
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = args.next().unwrap_or_else(|| "detection_set".into());
    let count: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);

    let font = SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    )
    .expect("font");

    let schema_source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/detection_sample.json"),
    )
    .expect("read schemas/detection_sample.json");
    let schema: serde_json::Value = serde_json::from_str(&schema_source).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema");

    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let texts: Vec<String> = TEXTS.iter().map(|s| s.to_string()).collect();

    let mut written = 0usize;
    let mut total_regions = 0usize;
    let mut by_enclosure = std::collections::BTreeMap::<&str, usize>::new();
    let mut degraded_pages = 0usize;

    for index in 0..count {
        // Seeded per page and derived from the index, so the whole set
        // regenerates identically. A training set you cannot reproduce is one
        // you cannot attribute a regression to.
        let mut rng = StdRng::seed_from_u64(0xC0FFEE + index as u64);

        // Density is swept deliberately across the set. `measure_recall` shows
        // recall falling as regions rise — 87.5% at 8 regions, 57.5% at 40 —
        // so a set clustered at one density would hide exactly the failure the
        // detector actually has.
        let target_regions = 6 + (index % 9) * 6;
        let spec = PageSpec {
            width: 1200,
            height: 1700,
            target_regions,
            ..Default::default()
        };

        let Ok((page, truth)) = render_page(&spec, &font, &texts, &mut rng) else {
            continue;
        };

        // Half the set carries scan degradation. A detector trained only on
        // clean renders learns clean renders; real pages are JPEG, rotated and
        // blurred, and that is the distribution it has to work in.
        let degrade_this = index % 2 == 1;
        let page = if degrade_this {
            let spec = DegradeSpec::sample_at(&mut rng, ScanQuality::Typical);
            match degrade_apply(&page, &spec, &mut rng) {
                Ok(degraded) => {
                    degraded_pages += 1;
                    degraded
                }
                // A degradation that fails is not a page to silently drop --
                // keep the clean render and let the summary's clean/degraded
                // split show it, rather than losing the sample.
                Err(_) => page,
            }
        } else {
            page
        };

        let stem = format!("page-{index:04}");
        let image_path = format!("{out_dir}/{stem}.png");
        page.save(&image_path).expect("save page");

        let regions: Vec<_> = truth
            .regions
            .iter()
            .map(|region| {
                let (ex, ey, ew, eh) = region.enclosure_bounds();
                *by_enclosure.entry(enclosure_name(&region.enclosure)).or_insert(0) += 1;
                let Some(direction) = direction_name(&region.direction) else {
                    panic!(
                        "unmapped writing mode {:?} — teach direction_name rather than \
                         emitting a label that fails its own schema",
                        region.direction
                    );
                };
                json!({
                    "index": region.index,
                    "box": {"x": region.x, "y": region.y, "width": region.width, "height": region.height},
                    "enclosure_box": {"x": ex, "y": ey, "width": ew, "height": eh},
                    "enclosure": enclosure_name(&region.enclosure),
                    "direction": direction,
                    "panel": region.panel,
                    "text": region.text,
                })
            })
            .collect();

        total_regions += regions.len();

        let sample = json!({
            "version": "1.0",
            "source": "synthetic",
            "page": {"path": format!("{stem}.png"), "width": truth.width, "height": truth.height},
            "panels": truth.panels.iter().map(|panel| json!({
                "x": panel.x, "y": panel.y, "width": panel.width, "height": panel.height
            })).collect::<Vec<_>>(),
            "regions": regions,
        });
        // Validated before writing, not after. A generator that can emit a
        // sample its own schema rejects will do so quietly at scale, and a
        // training set is exactly where a silently wrong label is most
        // expensive: the model learns it.
        if let Err(error) = validator.validate(&sample) {
            panic!("{stem} does not conform to schemas/detection_sample.json: {error}");
        }
        std::fs::write(
            format!("{out_dir}/{stem}.json"),
            serde_json::to_string_pretty(&sample).expect("serialize"),
        )
        .expect("write sample");
        written += 1;
    }

    // What the set contains, stated rather than left to be counted later. The
    // region count is what was DRAWN, not what was asked for: placement stops
    // early when a page fills up, so target and actual diverge on dense pages
    // and the difference is the interesting part.
    let summary = json!({
        "pages": written,
        "regions": total_regions,
        "regions_by_enclosure": by_enclosure,
        "degraded_pages": degraded_pages,
        "clean_pages": written - degraded_pages,
        "schema": "schemas/detection_sample.json",
    });
    std::fs::write(
        format!("{out_dir}/summary.json"),
        serde_json::to_string_pretty(&summary).expect("serialize summary"),
    )
    .expect("write summary");

    println!("wrote {written} pages, {total_regions} regions, to {out_dir}/");
    println!("  by enclosure: {by_enclosure:?}");
    println!(
        "  degraded {degraded_pages} / clean {}",
        written - degraded_pages
    );
}
