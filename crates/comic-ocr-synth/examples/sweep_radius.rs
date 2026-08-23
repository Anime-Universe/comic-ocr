//! Does the best merge_radius depend on page density?
use comic_ocr_synth::detect::{DetectSpec, detect_regions, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
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

    println!(
        "{:>8} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "density", "r=3", "r=4", "r=6", "r=8", "r=10"
    );
    println!("{}", "-".repeat(52));
    for target in [8usize, 16, 24, 40, 60] {
        let mut row = format!("{target:>8}");
        let mut best = (0u8, 0.0f32);
        for radius in [3u8, 4, 6, 8, 10] {
            let mut recalls = Vec::new();
            for seed in 0..3u64 {
                let mut rng = StdRng::seed_from_u64(4242 + seed);
                let spec = PageSpec {
                    width: 1200,
                    height: 1700,
                    target_regions: target,
                    ..Default::default()
                };
                let Ok((page, truth)) = render_page(&spec, &font, &texts, &mut rng) else {
                    continue;
                };
                let boxes: Vec<_> = truth
                    .regions
                    .iter()
                    .map(|r| {
                        let (x, y, w, h) = r.enclosure_bounds();
                        comic_ocr_synth::detect::Box2 {
                            x,
                            y,
                            width: w,
                            height: h,
                        }
                    })
                    .collect();
                let found = detect_regions(
                    &page,
                    &DetectSpec {
                        merge_radius: radius,
                        ..Default::default()
                    },
                );
                recalls.push(score(&boxes, &found, 0.5).recall);
            }
            let mean = recalls.iter().sum::<f32>() / recalls.len().max(1) as f32;
            if mean > best.1 {
                best = (radius, mean);
            }
            row.push_str(&format!(" {:>6.1}%", 100.0 * mean));
        }
        println!("{row}    best r={} ({:.1}%)", best.0, 100.0 * best.1);
    }
}
