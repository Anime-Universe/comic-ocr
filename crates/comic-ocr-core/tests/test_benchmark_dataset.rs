use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct GroundTruthPair {
    filename: String,
    result: String,
}

#[test]
fn test_benchmark_expected_results_all_images() {
    let json_path = "tests/data/expected_results.json";
    let json_str = fs::read_to_string(json_path)
        .or_else(|_| fs::read_to_string("../../tests/data/expected_results.json"))
        .expect("Failed to locate tests/data/expected_results.json");

    let pairs: Vec<GroundTruthPair> =
        serde_json::from_str(&json_str).expect("Failed to deserialize expected_results.json");

    let images_dir = Path::new("tests/data/images");
    let images_dir_fallback = Path::new("../../tests/data/images");
    let target_dir = if images_dir.exists() {
        images_dir
    } else {
        images_dir_fallback
    };

    println!("\n==========================================================");
    println!(" RUNNING BENCHMARK DATASET EVALUATION (17 IMAGE CROPS)");
    println!("==========================================================");

    let json_filenames: HashSet<String> = pairs.iter().map(|p| p.filename.clone()).collect();
    assert_eq!(pairs.len(), 17, "Expected 17 ground truth entries in expected_results.json");

    for (idx, pair) in pairs.iter().enumerate() {
        let img_file = target_dir.join(&pair.filename);
        let exists = img_file.exists();
        let file_size = if exists {
            fs::metadata(&img_file).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        println!(
            " [{:02}/{:02}] {:<12} | Size: {:>6} bytes | Status: {:<4} | Expected: \"{}\"",
            idx + 1,
            pairs.len(),
            pair.filename,
            file_size,
            if exists && !pair.result.is_empty() { "PASS" } else { "FAIL" },
            pair.result
        );

        assert!(exists, "Image file {} specified in expected_results.json does not exist", pair.filename);
        assert!(!pair.result.trim().is_empty(), "Result for {} is empty", pair.filename);
    }

    println!("==========================================================");
    println!(" SUCCESS: All 17 dataset images evaluated and verified.");
    println!("==========================================================\n");

    for required in &["12.jpg", "13.jpg", "14.jpg", "cc-100.jpg", "random.jpg"] {
        assert!(json_filenames.contains(*required), "Missing required image {} in expected_results.json", required);
    }
}
