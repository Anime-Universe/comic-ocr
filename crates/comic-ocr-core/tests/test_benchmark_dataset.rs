use comic_ocr_core::OcrEngine;
use comic_ocr_ort::OrtEngine;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BenchmarkRecord {
    filename: String,
    size_bytes: u64,
    status: String,
    expected_text: String,
    /// What this label IS, so the aggregate is not quietly wrong.
    ///
    /// `crop` — one text region, which is what this model reads.
    /// `page` — several regions concatenated. Scoring a single-crop reader
    ///          against a whole page measures the benchmark's framing, not the
    ///          model: it reads one region correctly and is marked ~100% wrong.
    /// `degenerate` — the label is an ellipsis. Matching it scores 0% CER and
    ///          proves nothing, so including it flatters every mean it enters.
    #[serde(default = "default_label_kind")]
    label_kind: String,
    actual_text: String,
    cer_divergence: f64,
}

/// Computes Character Error Rate (CER) via Levenshtein distance normalized by reference length.
fn default_label_kind() -> String {
    // Entries written before this field existed were all single crops.
    "crop".to_string()
}

pub fn compute_cer(expected: &str, actual: &str) -> f64 {
    let exp_chars: Vec<char> = expected.chars().collect();
    let act_chars: Vec<char> = actual.chars().collect();

    if exp_chars.is_empty() {
        return if act_chars.is_empty() { 0.0 } else { 1.0 };
    }

    let m = exp_chars.len();
    let n = act_chars.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if exp_chars[i - 1] == act_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    let dist = dp[m][n] as f64;
    dist / (m as f64)
}

#[test]
fn test_benchmark_schema_integrity() {
    let json_path = "tests/data/benchmark_results.json";
    let json_str = fs::read_to_string(json_path)
        .or_else(|_| fs::read_to_string("../../tests/data/benchmark_results.json"))
        .expect("Failed to locate tests/data/benchmark_results.json");

    let records: Vec<BenchmarkRecord> =
        serde_json::from_str(&json_str).expect("Failed to deserialize benchmark_results.json");

    let images_dir = Path::new("tests/data/images");
    let images_dir_fallback = Path::new("../../tests/data/images");
    let target_dir = if images_dir.exists() {
        images_dir
    } else {
        images_dir_fallback
    };

    let json_filenames: HashSet<String> = records.iter().map(|p| p.filename.clone()).collect();
    assert!(
        records.len() >= 17,
        "Expected at least 17 benchmark entries in benchmark_results.json"
    );

    for record in &records {
        let img_file = target_dir.join(&record.filename);
        assert!(
            img_file.exists(),
            "Image file {} specified in benchmark_results.json does not exist",
            record.filename
        );
        assert!(
            !record.expected_text.trim().is_empty(),
            "Expected text for {} is empty",
            record.filename
        );

        // Assert CER math consistency
        let calculated_cer = compute_cer(&record.expected_text, &record.actual_text);
        assert!(
            (calculated_cer - record.cer_divergence).abs() < 1e-4,
            "CER mismatch for {}",
            record.filename
        );
    }

    for required in &["12.jpg", "13.jpg", "14.jpg", "cc-100.jpg", "random.jpg"] {
        assert!(
            json_filenames.contains(*required),
            "Missing required image {} in benchmark_results.json",
            required
        );
    }
}

#[test]
#[ignore = "Dynamic ONNX benchmark runner (run with cargo test --test test_benchmark_dataset -- --ignored --nocapture)"]
fn test_benchmark_model_inference_evaluation() {
    let json_path = "tests/data/benchmark_results.json";
    let json_str = fs::read_to_string(json_path)
        .or_else(|_| fs::read_to_string("../../tests/data/benchmark_results.json"))
        .expect("Failed to locate tests/data/benchmark_results.json");

    let records: Vec<BenchmarkRecord> =
        serde_json::from_str(&json_str).expect("Failed to deserialize benchmark_results.json");

    let images_dir = Path::new("tests/data/images");
    let images_dir_fallback = Path::new("../../tests/data/images");
    let target_dir = if images_dir.exists() {
        images_dir
    } else {
        images_dir_fallback
    };

    let model_name =
        std::env::var("COMIC_OCR_MODEL").unwrap_or_else(|_| "kha-white/manga-ocr-base".to_string());
    let engine = OrtEngine::new(model_name);

    println!("\n==========================================================");
    println!(" RUNNING DYNAMIC INFERENCE BENCHMARK EVALUATION (17 IMAGES)");
    println!("==========================================================");

    // Scored over CROPS only. `page` labels concatenate several regions and this
    // model reads one; `degenerate` labels are an ellipsis and matching one
    // proves nothing. Both are still RUN and printed — an excluded case that
    // disappears from the output is indistinguishable from one that passed —
    // but they do not enter the mean, because a mean over three different
    // questions answers none of them.
    let mut total_cer = 0.0f64;
    let mut scored = 0usize;
    let mut excluded = 0usize;
    for (idx, record) in records.iter().enumerate() {
        let img_path = target_dir.join(&record.filename);
        let img = image::open(&img_path)
            .unwrap_or_else(|_| panic!("Failed to open image {}", img_path.display()));

        let ocr_result = engine
            .predict(&img)
            .unwrap_or_else(|e| panic!("OCR prediction failed for {}: {}", record.filename, e));

        let cleaned_pred = comic_ocr_core::post_process_jp(&ocr_result.text, false);
        let cer = compute_cer(&record.expected_text, &cleaned_pred);
        if record.label_kind == "crop" {
            total_cer += cer;
            scored += 1;
        } else {
            excluded += 1;
        }

        println!(
            " [{:02}/{:02}] {:<12} | Expected: \"{}\" | Predicted: \"{}\" | CER: {:.2}%",
            idx + 1,
            records.len(),
            record.filename,
            record.expected_text,
            cleaned_pred,
            cer * 100.0
        );
    }

    // Zero scored is not a pass. An empty mean is 0.0, which would sail through
    // any threshold while measuring nothing at all.
    assert!(
        scored > 0,
        "no crop-labelled records were scored — the benchmark measured nothing"
    );
    let avg_cer = total_cer / scored as f64;
    println!(
        "\n  scored {scored} crop(s); {excluded} excluded (page or degenerate labels, run and printed above)"
    );
    println!("  mean CER over crops: {:.2}%", avg_cer * 100.0);
    assert!(
        avg_cer <= 0.20,
        "Mean CER over {scored} crop label(s) exceeded the 20% tolerance: {:.2}%",
        avg_cer * 100.0
    );
}
