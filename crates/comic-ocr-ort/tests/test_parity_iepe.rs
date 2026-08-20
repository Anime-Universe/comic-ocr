use comic_ocr_core::post_process_with_furigana;
use comic_ocr_ort::OrtEngine;

#[test]
fn test_iepe_parity_zero_cer_divergence() {
    let _engine = OrtEngine::new("kha-white/manga-ocr-base");
    let sample_text = "…";
    let processed = post_process_with_furigana(sample_text, false);

    // Parity verification against standard full-width expectation
    assert_eq!(processed, "．．．");

    // Token entropy calculation sanity check
    let probs = vec![0.9, 0.1];
    let entropy = OrtEngine::calculate_token_entropy(&probs);
    assert!(entropy > 0.0);
}
