use comic_ocr_core::*;
use std::fs;

#[test]
fn test_schema_json_suite_deserialization() {
    // 1. Comic Scene Graph
    let scene_json = fs::read_to_string("schemas/examples/sample_comic_scene_graph.json")
        .or_else(|_| fs::read_to_string("../../schemas/examples/sample_comic_scene_graph.json"))
        .expect("Failed to read sample_comic_scene_graph.json");
    let doc: MangaDocument =
        serde_json::from_str(&scene_json).expect("Failed to parse sample_comic_scene_graph.json");
    assert_eq!(doc.metadata.source_language, "ja");
    assert_eq!(doc.pages.len(), 1);

    // 2. Localized Text Object
    let text_obj_json = fs::read_to_string("schemas/examples/sample_localized_text_object.json")
        .or_else(|_| fs::read_to_string("../../schemas/examples/sample_localized_text_object.json"))
        .expect("Failed to read sample_localized_text_object.json");
    let text_obj: serde_json::Value = serde_json::from_str(&text_obj_json)
        .expect("Failed to parse sample_localized_text_object.json");
    assert_eq!(text_obj["role"], "dialogue");

    // 3. OCR Result
    let ocr_json = fs::read_to_string("schemas/examples/sample_ocr_result.json")
        .or_else(|_| fs::read_to_string("../../schemas/examples/sample_ocr_result.json"))
        .expect("Failed to read sample_ocr_result.json");
    let ocr_res: OcrResult =
        serde_json::from_str(&ocr_json).expect("Failed to parse sample_ocr_result.json");
    assert_eq!(ocr_res.text, "立川で見た〝穴〟の下の巨大な眼は:");

    // 4. Page Result
    let page_json = fs::read_to_string("schemas/examples/sample_page_result.json")
        .or_else(|_| fs::read_to_string("../../schemas/examples/sample_page_result.json"))
        .expect("Failed to read sample_page_result.json");
    let page_res: serde_json::Value =
        serde_json::from_str(&page_json).expect("Failed to parse sample_page_result.json");
    assert_eq!(page_res["panels"][0]["reading_order"], 1);

    // 5. PDP Decision
    let pdp_json = fs::read_to_string("schemas/examples/sample_pdp_decision.json")
        .or_else(|_| fs::read_to_string("../../schemas/examples/sample_pdp_decision.json"))
        .expect("Failed to read sample_pdp_decision.json");
    let pdp_res: serde_json::Value =
        serde_json::from_str(&pdp_json).expect("Failed to parse sample_pdp_decision.json");
    assert_eq!(pdp_res["is_validated"], true);
}
