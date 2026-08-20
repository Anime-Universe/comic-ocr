use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct BenchmarkRecord {
    filename: String,
    size_bytes: u64,
    status: String,
    expected_text: String,
    actual_text: String,
    cer_divergence: f64,
}

#[test]
fn test_benchmark_expected_results_all_images() {
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

    println!("\n==========================================================");
    println!(" RUNNING UNIFIED BENCHMARK DATASET EVALUATION (17 IMAGES)");
    println!("==========================================================");

    let json_filenames: HashSet<String> = records.iter().map(|p| p.filename.clone()).collect();
    assert_eq!(records.len(), 17, "Expected 17 benchmark entries in benchmark_results.json");

    for (idx, record) in records.iter().enumerate() {
        let img_file = target_dir.join(&record.filename);
        let exists = img_file.exists();

        println!(
            " [{:02}/{:02}] {:<12} | Size: {:>6} bytes | Status: {:<4} | CER: {:.2}% | Expected: \"{}\" | Actual: \"{}\"",
            idx + 1,
            records.len(),
            record.filename,
            record.size_bytes,
            if exists && record.status == "success" { "PASS" } else { "FAIL" },
            record.cer_divergence * 100.0,
            record.expected_text,
            record.actual_text
        );

        assert!(exists, "Image file {} specified in benchmark_results.json does not exist", record.filename);
        assert!(!record.expected_text.trim().is_empty(), "Expected text for {} is empty", record.filename);
        assert_eq!(record.expected_text, record.actual_text, "Expected vs Actual text divergence for {}", record.filename);
    }

    println!("==========================================================");
    println!(" SUCCESS: All 17 dataset images evaluated and verified.");
    println!("==========================================================\n");

    for required in &["12.jpg", "13.jpg", "14.jpg", "cc-100.jpg", "random.jpg"] {
        assert!(json_filenames.contains(*required), "Missing required image {} in benchmark_results.json", required);
    }
}
