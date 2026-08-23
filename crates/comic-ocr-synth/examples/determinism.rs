//! Is the engine deterministic on a byte-identical image?
//!
//! Two of my probes disagreed at nominally identical geometry. Before blaming
//! the geometry I have to rule out the harness: if the same image read twice
//! gives two answers, every measurement in this crate -- and the benchmark, and
//! anything built on it -- is provisional for a reason that has nothing to do
//! with rendering.
use comic_ocr_core::types::OcrEngine as _;
use comic_ocr_synth::render::{Direction, RenderSpec, SynthFont, render};

fn main() {
    let font = SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    )
    .expect("font");
    let engine = comic_ocr_ort::OrtEngine::new("determinism");
    if engine.generator.is_none() {
        eprintln!("no generator; set COMIC_OCR_ONNX_DIR");
        std::process::exit(2);
    }

    let spec = RenderSpec {
        text: "第30話重苦しい闇の奥".into(),
        direction: Direction::HorizontalTb,
        font_px: 37.0,
        cells_per_run: 6,
        ..Default::default()
    };
    let img = render(&spec, &font).expect("render");
    let dyn_img = image::DynamicImage::ImageLuma8(img);

    let mut seen: Vec<(String, f32)> = Vec::new();
    for i in 0..8 {
        let out = engine.predict(&dyn_img).expect("predict");
        println!("  run {i}: conf {:.6}  {}", out.confidence, out.text);
        seen.push((out.text, out.confidence));
    }
    let distinct: std::collections::BTreeSet<&String> = seen.iter().map(|(t, _)| t).collect();
    println!(
        "\n{} distinct readings over 8 runs of ONE image -> engine is {}",
        distinct.len(),
        if distinct.len() == 1 {
            "DETERMINISTIC"
        } else {
            "NON-DETERMINISTIC"
        }
    );
}
