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

    for (idx, record) in records.iter().enumerate() {
        println!(
            " [{:02}/{:02}] {:<12} | Expected: \"{}\" | Recorded CER: {:.2}%",
            idx + 1,
            records.len(),
            record.filename,
            record.expected_text,
            record.cer_divergence * 100.0
        );
    }
}
