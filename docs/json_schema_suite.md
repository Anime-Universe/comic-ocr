# JSON Schema Suite & Comprehensive 12.jpg Execution Reference

**Document Version:** `v1.0.0`  
**Protocol Status:** `Normative Schema Reference`  

---

## 1. Schema Suite Overview

This repository publishes 7 canonical JSON Schema contracts under [`schemas/`](../schemas/):

1. **`comic_scene_graph.json`**: Authoritative 4-layer scene graph specification (`ComicDocument` / `MangaDocument`).
2. **`localized_text_object.json`**: Compact compiled runtime payload for individual text objects (`LocalizedTextObject`).
3. **`ocr_result.json`**: Raw OCR inference result schema (`OcrResult`).
4. **`page_result.json`**: Full-page bubble & panel OCR structure (`PageResult`).
5. **`pdp_decision.json`**: Polymorphic Decision Protocol engine consensus score & candidate ledger (`PdpDecision`).
6. **`ipub_semantic_resource.json`**: iPub Semantic Resource (`IPubSemanticResource`) envelope schema for page-semantics.
7. **`training_pair.json`**: (Crop, Text) training pair schema with composed confidence ($\mathbf{C}_{\text{pair}} = \mathbf{C}_{\text{det}} \times \mathbf{C}_{\text{trans}}$).

---

## 2. Sample Suite Directory (`schemas/examples/`)

Corresponding validated sample JSON instances are maintained under [`schemas/examples/`](../schemas/examples/):

- [`sample_comic_scene_graph.json`](../schemas/examples/sample_comic_scene_graph.json)
- [`sample_localized_text_object.json`](../schemas/examples/sample_localized_text_object.json)
- [`sample_ocr_result.json`](../schemas/examples/sample_ocr_result.json)
- [`sample_page_result.json`](../schemas/examples/sample_page_result.json)
- [`sample_pdp_decision.json`](../schemas/examples/sample_pdp_decision.json)
- [`sample_ipub_semantic_resource.json`](../schemas/examples/sample_ipub_semantic_resource.json)

---

## 3. Comprehensive Execution Result (`assets/examples/12.jpg`)

Running `assets/examples/12.jpg` through the complete 4-layer pipeline generates the multi-schema document saved at [`tests/data/12_comprehensive_run_result.json`](../tests/data/12_comprehensive_run_result.json):

```json
{
  "input_file": "assets/examples/12.jpg",
  "image_dimensions": {
    "width": 1024,
    "height": 793,
    "size_bytes": 201986
  },
  "recognized_text": "ああ、",
  "ocr_result": {
    "text": "ああ、",
    "confidence": 0.985,
    "token_probabilities": [0.99, 0.985, 0.98, 0.985],
    "metadata": {
      "duration_ms": 4.2,
      "model_name": "the reference checkpoint",
      "engine_type": "BaseInt8Onnx"
    }
  },
  "pdp_decision": {
    "selected_text": "ああ、",
    "confidence": 0.985,
    "is_validated": true,
    "candidates": [
      {
        "engine_type": "BaseInt8Onnx",
        "text": "ああ、",
        "raw_confidence": 0.985,
        "acs_score": 0.99
      }
    ]
  },
  "localized_text_object": {
    "role": "dialogue",
    "source": {
      "language": "ja",
      "raw": "ああ、",
      "normalized": "ああ、",
      "reading": "ああ、"
    },
    "translation": {
      "language": "en",
      "literal": "Ah,",
      "localized": "Ah...",
      "displayText": "Ah..."
    }
  }
}
```

---

## 4. Build & Execution Protocol

```bash
# 1. Cargo Test Workspace
cargo test --workspace

# 2. Run CLI with --json output
cargo run -p comic-ocr-cli -- --image assets/examples/12.jpg --json
```
