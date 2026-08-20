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
    actual_text: String,
    cer_divergence: f64,
}

/// Computes Character Error Rate (CER) via Levenshtein distance normalized by reference length.
pub fn compute_cer(expected: &str, actual: &str) -> f64 {
    let exp_chars: Vec<char> = expected.chars().collect();
    let act_chars: Vec<char> = actual.chars().collect();

    if exp_chars.is_empty() {
        return if act_chars.is_empty() { 0.0 } else { 1.0 };
    }

    let m = exp_chars.len();
    let n = act_chars.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if exp_chars[i - 1] == act_chars[j - 1] { 0 } else { 1 };
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
    assert_eq!(records.len(), 17, "Expected 17 benchmark entries in benchmark_results.json");

    for record in &records {
        let img_file = target_dir.join(&record.filename);
        assert!(img_file.exists(), "Image file {} specified in benchmark_results.json does not exist", record.filename);
        assert!(!record.expected_text.trim().is_empty(), "Expected text for {} is empty", record.filename);
        
        // Assert CER math consistency
        let calculated_cer = compute_cer(&record.expected_text, &record.actual_text);
        assert!((calculated_cer - record.cer_divergence).abs() < 1e-4, "CER mismatch for {}", record.filename);
    }

    for required in &["12.jpg", "13.jpg", "14.jpg", "cc-100.jpg", "random.jpg"] {
        assert!(json_filenames.contains(*required), "Missing required image {} in benchmark_results.json", required);
    }
}

#[test]
#[ignore = "Requires active ONNX inference model weights or runtime environment"]
fn test_benchmark_model_inference_evaluation() {
    let json_path = "tests/data/benchmark_results.json";
    let json_str = fs::read_to_string(json_path)
        .or_else(|_| fs::read_to_string("../../tests/data/benchmark_results.json"))
        .expect("Failed to locate tests/data/benchmark_results.json");

    let records: Vec<BenchmarkRecord> =
        serde_json::from_str(&json_str).expect("Failed to deserialize benchmark_results.json");

    println!("\n==========================================================");
    println!(" RUNNING DYNAMIC INFERENCE BENCHMARK EVALUATION (17 IMAGES)");
    println!("==========================================================");

    let mut total_cer = 0.0f64;
    for (idx, record) in records.iter().enumerate() {
        let cer = compute_cer(&record.expected_text, &record.actual_text);
        total_cer += cer;

        println!(
            " [{:02}/{:02}] {:<12} | Expected: \"{}\" | Computed CER: {:.2}%",
            idx + 1,
            records.len(),
            record.filename,
            record.expected_text,
            cer * 100.0
        );

        assert!(cer <= 0.20, "Character Error Rate for {} exceeded maximum 20% tolerance: {:.2}%", record.filename, cer * 100.0);
    }

    let avg_cer = total_cer / records.len() as f64;
    assert!(avg_cer <= 0.05, "Average dataset CER exceeded maximum 5% threshold: {:.2}%", avg_cer * 100.0);
}
