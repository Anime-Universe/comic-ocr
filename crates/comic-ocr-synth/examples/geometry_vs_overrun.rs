//! Does the same label overrun under one layout and stop cleanly under another?
//!
//! probe_termination found ZERO overruns at a fixed 32px / 8-per-column layout.
//! compare_confidence found several at scales and directions matched to real
//! crops. Same model, same labels, same font -- so either one probe is wrong or
//! overrun is a function of the rendering, which would contradict "the cause is
//! not in the input image".
use comic_ocr_core::types::OcrEngine as _;
use comic_ocr_synth::render::{Direction, RenderSpec, SynthFont, render};

fn main() {
    let font_path = std::env::var("COMIC_OCR_SYNTH_FONT")
        .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into());
    let font = SynthFont::from_path(&font_path, 0).expect("font");
    let engine = comic_ocr_ort::OrtEngine::new("geom");
    if engine.generator.is_none() {
        eprintln!("no generator; set COMIC_OCR_ONNX_DIR");
        std::process::exit(2);
    }

    let labels = [
        "警察にも先生にも町中の",
        "第30話重苦しい闇の奥",
        "LINK!私達7人の力",
    ];
    // cells_per_run is characters PER run, not the number of runs.
    // compare_confidence printed the run count, so the layouts it actually used
    // for an 11-character label were ceil(11/runs) per run -- 6 for 2 runs,
    // 4 for 3 runs. Reproducing those exactly is the whole point here.
    let geoms: [(f32, usize, Direction); 7] = [
        (32.0, 8, Direction::VerticalRl),   // probe_termination's layout
        (37.0, 6, Direction::HorizontalTb), // compare_confidence: 第30話, H 37 x2 runs
        (26.0, 6, Direction::HorizontalTb), // compare_confidence: LINK!,  H 26 x2 runs
        (42.0, 4, Direction::VerticalRl),   // compare_confidence: 警察,   V 42 x3 runs
        (64.0, 6, Direction::HorizontalTb),
        (21.0, 5, Direction::VerticalRl),
        (52.0, 11, Direction::VerticalRl),
    ];

    for label in labels {
        if !font.uncovered(label).is_empty() {
            continue;
        }
        let n = label.chars().count();
        println!("\n{label}  ({n} chars)");
        for (px, per_run, dir) in geoms {
            let spec = RenderSpec {
                text: label.into(),
                direction: dir,
                font_px: px,
                cells_per_run: per_run,
                ..Default::default()
            };
            let Ok(img) = render(&spec, &font) else {
                continue;
            };
            let (w, h) = (img.width(), img.height());
            let Ok(out) = engine.predict(&image::DynamicImage::ImageLuma8(img)) else {
                continue;
            };
            let over = out.text.chars().count() as i32 - n as i32;
            let tag = if out.text == label {
                "exact"
            } else if out.text.starts_with(label) {
                "OVERRUN"
            } else {
                "misread"
            };
            println!(
                "  {}{:>3.0}px x{:<2} {:>4}x{:<4} {:>7} {:>+3}  conf {:.3}  {}",
                if dir == Direction::VerticalRl {
                    "V"
                } else {
                    "H"
                },
                px,
                per_run,
                w,
                h,
                tag,
                over,
                out.confidence,
                out.text.chars().take(20).collect::<String>()
            );
        }
    }
}
