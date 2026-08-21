//! Read every labelled test page and score it against the expected text.
//!
//! An example rather than a test: it needs a model directory and ~554 MB of
//! graphs, and a test that silently skips without them is indistinguishable
//! from one that passes.
//!
//!   COMIC_OCR_ONNX_DIR=models/onnx cargo run -p comic-ocr-ort --example eval_corpus
use comic_ocr_core::types::OcrEngine as _;

/// Character error rate: edit distance over the expected length.
fn cer(expected: &str, actual: &str) -> f64 {
    let a: Vec<char> = expected.chars().collect();
    let b: Vec<char> = actual.chars().collect();
    if a.is_empty() {
        return if b.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let sub = prev[j - 1] + usize::from(a[i - 1] != b[j - 1]);
            cur[j] = sub.min(prev[j] + 1).min(cur[j - 1] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] as f64 / a.len() as f64
}

fn main() {
    let root = std::env::var("COMIC_OCR_CORPUS")
        .unwrap_or_else(|_| "/Users/zachshallbetter/Projects/comic-ocr-rust".to_string());
    let labels: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{root}/tests/data/benchmark_results.json"))
            .expect("benchmark_results.json"),
    )
    .expect("valid json");

    let engine = comic_ocr_ort::OrtEngine::new("eval");
    if engine.generator.is_none() {
        eprintln!("no model directory loaded — set COMIC_OCR_ONNX_DIR");
        std::process::exit(2);
    }

    let entries = labels.as_array().expect("a list of entries");
    let (mut scored, mut exact, mut total_cer, mut failed) = (0usize, 0usize, 0.0f64, 0usize);

    println!("{:<10} {:>7} {:>7}  reading", "page", "CER", "conf");
    println!("{}", "-".repeat(78));
    for entry in entries {
        let name = entry["filename"].as_str().unwrap_or("?");
        let expected = entry["expected_text"].as_str().unwrap_or("");
        let path = format!("{root}/tests/data/images/{name}");
        let Ok(img) = image::open(&path) else {
            println!("{name:<10} {:>7} {:>7}  (image unreadable)", "—", "—");
            failed += 1;
            continue;
        };
        match engine.predict(&img) {
            Ok(out) => {
                let e = cer(expected, &out.text);
                scored += 1;
                total_cer += e;
                if e == 0.0 {
                    exact += 1;
                }
                println!(
                    "{name:<10} {:>6.1}% {:>7.4}  {}",
                    e * 100.0,
                    out.confidence,
                    out.text.chars().take(30).collect::<String>()
                );
            }
            Err(err) => {
                println!("{name:<10} {:>7} {:>7}  FAILED: {err}", "—", "—");
                failed += 1;
            }
        }
    }
    println!("{}", "-".repeat(78));
    if scored > 0 {
        println!(
            "scored {scored}/{} · exact {exact} · mean CER {:.2}% · failed {failed}",
            entries.len(),
            (total_cer / scored as f64) * 100.0
        );
    } else {
        // Zero scored is not a good result; it is no result.
        println!("scored NOTHING — {failed} failure(s). This is not a passing run.");
    }
}
