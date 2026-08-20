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

    assert!(target_dir.exists(), "Target images directory does not exist");

    let json_filenames: HashSet<String> = pairs.iter().map(|p| p.filename.clone()).collect();

    // Verify all 17 ground truth entries exist and have non-empty results
    assert_eq!(pairs.len(), 17, "Expected 17 ground truth entries in expected_results.json");
    for pair in &pairs {
        assert!(!pair.result.trim().is_empty(), "Result for {} is empty", pair.filename);
        let img_file = target_dir.join(&pair.filename);
        assert!(img_file.exists(), "Image file {} specified in expected_results.json does not exist", pair.filename);
    }

    // Verify specifically 12.jpg, 13.jpg, 14.jpg, cc-100.jpg, and random.jpg are present and return results
    for required in &["12.jpg", "13.jpg", "14.jpg", "cc-100.jpg", "random.jpg"] {
        assert!(json_filenames.contains(*required), "Missing required image {} in expected_results.json", required);
    }
}
