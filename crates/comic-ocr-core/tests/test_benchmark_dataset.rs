use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct GroundTruthPair {
    filename: String,
    result: String,
}

#[test]
fn test_benchmark_expected_results_dataset() {
    let path = "tests/data/expected_results.json";
    let json_str = fs::read_to_string(path)
        .or_else(|_| fs::read_to_string("../../tests/data/expected_results.json"))
        .expect("Failed to locate tests/data/expected_results.json");

    let pairs: Vec<GroundTruthPair> =
        serde_json::from_str(&json_str).expect("Failed to deserialize expected_results.json");

    assert_eq!(pairs.len(), 12);
    assert_eq!(pairs[0].filename, "00.jpg");
    assert_eq!(pairs[0].result, "素直にあやまるしか");
    assert_eq!(pairs[11].filename, "11.jpg");
    assert_eq!(pairs[11].result, "警察にも先生にも町中の人達に！！");
}
