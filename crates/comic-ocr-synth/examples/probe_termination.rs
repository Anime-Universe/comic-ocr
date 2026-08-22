//! Is the decoder's failure to stop content-dependent, or flat?
//!
//! Asked by `infinite-verse-c3` while assessing Infinite-Verse#116/#837/#859.
//! The distinction decides what kind of defect it is:
//!
//!   flat across content     -> a stopping-criterion bug
//!   scales with content     -> a budget-versus-content story, in a second model
//!
//! And a second question, which is the stronger test: are there crops where the
//! model reads the label WRONG and stops cleanly? If overrun only ever appears
//! alongside a correct read, "it does not know when to stop" is a much weaker
//! claim than it sounds -- the model might simply be continuing plausible text.
//!
//! Labels come from the real benchmark corpus and from concatenations of it, so
//! the character distribution and the language model prior stay realistic.
//! Random glyph strings would confound the answer with out-of-distribution text.
//!
//!   COMIC_OCR_ONNX_DIR=... cargo run -p comic-ocr-synth --example probe_termination

use comic_ocr_core::types::OcrEngine as _;
use comic_ocr_synth::render::{Direction, RenderSpec, SynthFont, render};

struct Sample {
    label_len: usize,
    out_len: usize,
    prefix_ok: bool,
    exact: bool,
}

fn main() {
    let root = std::env::var("COMIC_OCR_CORPUS")
        .unwrap_or_else(|_| "/Users/zachshallbetter/Projects/comic-ocr-rust".into());
    let font_path = std::env::var("COMIC_OCR_SYNTH_FONT")
        .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into());
    let Ok(font) = SynthFont::from_path(&font_path, 0) else {
        eprintln!("no font at {font_path}");
        std::process::exit(2);
    };
    let engine = comic_ocr_ort::OrtEngine::new("terminate");
    if engine.generator.is_none() {
        eprintln!("no generator; set COMIC_OCR_ONNX_DIR");
        std::process::exit(2);
    }

    let raw = std::fs::read_to_string(format!("{root}/tests/data/benchmark_results.json"))
        .expect("benchmark_results.json");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("valid json");
    let base: Vec<String> = rows
        .iter()
        .filter(|r| r["label_kind"] == "crop")
        .filter_map(|r| r["expected_text"].as_str())
        .filter(|s| font.uncovered(s).is_empty())
        .map(str::to_string)
        .collect();

    if base.is_empty() {
        eprintln!("no usable labels");
        std::process::exit(1);
    }

    // Natural lengths from the corpus, plus concatenations so the long end is
    // still real language rather than noise.
    let mut labels: Vec<String> = base.clone();
    for w in 2..=4 {
        for chunk in base.chunks(w) {
            if chunk.len() == w {
                labels.push(chunk.concat());
            }
        }
    }

    let mut samples: Vec<Sample> = Vec::new();

    for label in &labels {
        let chars = label.chars().filter(|c| !c.is_whitespace()).count();
        // Keep geometry plausible: roughly square blocks, ~8 glyphs per column.
        let per_run = chars.clamp(1, 8);
        let spec = RenderSpec {
            text: label.clone(),
            direction: Direction::VerticalRl,
            font_px: 32.0,
            cells_per_run: per_run,
            ..Default::default()
        };
        let Ok(img) = render(&spec, &font) else {
            continue;
        };
        let Ok(out) = engine.predict(&image::DynamicImage::ImageLuma8(img)) else {
            continue;
        };
        let out_len = out.text.chars().count();
        samples.push(Sample {
            label_len: chars,
            out_len,
            prefix_ok: out.text.starts_with(label.as_str()),
            exact: out.text == *label,
        });
    }

    // Q1: does overrun scale with content?
    println!(
        "{:<12} {:>5} {:>9} {:>9} {:>9}",
        "label chars", "n", "mean out", "mean over", "exact"
    );
    println!("{}", "-".repeat(52));
    let buckets: [(usize, usize); 5] = [(1, 8), (9, 14), (15, 22), (23, 34), (35, 200)];
    for (lo, hi) in buckets {
        let b: Vec<&Sample> = samples
            .iter()
            .filter(|s| s.label_len >= lo && s.label_len <= hi)
            .collect();
        if b.is_empty() {
            continue;
        }
        let n = b.len() as f64;
        let mean_out = b.iter().map(|s| s.out_len as f64).sum::<f64>() / n;
        let mean_over = b
            .iter()
            .map(|s| s.out_len as f64 - s.label_len as f64)
            .sum::<f64>()
            / n;
        let exact = b.iter().filter(|s| s.exact).count();
        println!(
            "{lo:>3}-{hi:<8} {:>5} {mean_out:>9.1} {mean_over:>+9.1} {exact:>5}/{}",
            b.len(),
            b.len()
        );
    }

    // Q2: the stronger test -- wrong reads that stopped cleanly.
    let wrong: Vec<&Sample> = samples.iter().filter(|s| !s.prefix_ok).collect();
    let wrong_stopped = wrong.iter().filter(|s| s.out_len <= s.label_len).count();
    let right_over = samples
        .iter()
        .filter(|s| s.prefix_ok && s.out_len > s.label_len)
        .count();
    println!("\n{}", "-".repeat(52));
    println!("total crops                       {}", samples.len());
    println!("read label correctly, then overran {right_over}");
    println!("misread the label                  {}", wrong.len());
    println!("  of those, stopped at or under    {wrong_stopped}");
    println!(
        "\nIf overrun is flat across the length buckets, the decoder is not\n\
         stopping regardless of content. If it grows, content drives it.\n\
         Misreads that stop cleanly are the control: without them, overrun\n\
         may just be the model continuing plausible text rather than failing\n\
         to terminate."
    );
}
