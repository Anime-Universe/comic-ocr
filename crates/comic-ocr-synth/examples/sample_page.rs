//! Render one page with panels and balloons, for looking at.
//!
//! Counting tests and unit assertions both passed on a page that was a word
//! cloud, which is why this exists: some defects are only visible.
//!
//!   cargo run -p comic-ocr-synth --example sample_page -- out.png
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sample_page.png".into());
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

    let mut rng = StdRng::seed_from_u64(20260822);
    let spec = PageSpec {
        width: 1200,
        height: 1700,
        target_regions: 22,
        ..Default::default()
    };
    let (page, truth) = render_page(&spec, &font, &texts, &mut rng).expect("page");
    page.save(&out).expect("save");

    let balloons = truth
        .regions
        .iter()
        .filter(|r| format!("{:?}", r.enclosure) == "Balloon")
        .count();
    let in_panel = truth.regions.iter().filter(|r| r.panel.is_some()).count();
    println!(
        "{out}: {} regions ({balloons} balloons, {in_panel} inside a panel), {} panels",
        truth.region_count,
        truth.panels.len()
    );
}
