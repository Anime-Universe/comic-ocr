---
name: comic-ocr-expert
description: Operational instructions, architectural doctrine, quality verification gates, and roadmap for Japanese Manga & Comic OCR in Rust.
---

# Manga & Comic OCR Rust Expert Skill

This skill provides normative operational directives, architectural doctrine, and quality verification gates for maintaining and developing **Manga OCR Rust** and **Comic OCR Rust**.

---

## 1. Architectural Doctrine & Status Honesty

1. **Status Honesty & Error Propagation**:
   - An OCR engine prediction must return `Err(OcrError)` on failure.
   - **NEVER** return `Ok(OcrResult)` with hardcoded confidence (`0.985`) or fake text on a failed model execution. Silent fallback returns cause corpus/graph poisoning downstream.
2. **Provenance & Confidence Decoupling**:
   - Never launder multi-pass vision/semantic analysis confidence into raw single-crop OCR engine confidence.
   - Attach explicit `AnalysisEvidence` provenance records specifying `source` (`ocr`, `vision`, `geometry`, `metadata`, `language_model`, `human`, `fixture`), `confidence`, `pass_id`, and `derived_from`.
3. **Five-Layer Semantic Validation Engine**:
   - **OCR Truth**: Character recognizer validation.
   - **Detection Truth**: Region geometry coverage.
   - **Semantic Truth**: Role consistency (e.g. page badge vs. chapter number).
   - **Spatial Truth**: Geometry boundary checks.
   - **Reading Order Truth**: Automatic RTL manga spatial ordering consistency validation (`ReadingOrderValidation`).
4. **Separation of Translation vs. Role Metadata**:
   - Keep text translation strictly faithful to the source characters.
   - Do **NOT** fold inferred credit roles (e.g. `"Supervisor: Yuji Horii"`) into `translation.localized`. Use separate `credit` metadata objects (`credit: { type: "supervisor", labelSource: "adjacent-english-text" }`).
5. **Multilingual Cover Art & Preserved Regions**:
   - Non-translated text on manga covers (e.g. English logos `DRAGON QUEST`, volume numbers `1`, tags `WARRIORS`) are first-class `text_regions` with `strategy: "preserve"` and `language: "en"`.
6. **Canonical Scene Graph Integration**:
   - `comic_scene_graph` is the authoritative source of truth. All `text_regions`, `containers`, and `panels` must be fully populated inside `comic_scene_graph.pages[0]`.

---

## 2. Core Workspace Crates

- **`comic-ocr-core`**: Domain primitives, `OcrEngine` trait, Furigana bracket FSM, multi-tile resampling, row-bucket reading order sorting, protected art avoidance solver, auto-lettering solver, `validation` engine (`ReadingOrderValidation`, `SemanticAssertion`).
- **`comic-ocr-pdp`**: Multi-engine consensus evaluation, Brier-calibrated candidate selection, ACS score discounting.
- **`comic-ocr-ort`**: Native ONNX Runtime session execution (`from_onnx_file`), numerically stable 2D softmax, Shannon token entropy ($H_k$), rolling loop truncation check ($\bar{H} < 0.15$).
- **`comic-ocr-cli`**: High-performance operational CLI (`comic-ocr`).
- **`comic-ocr-runtime`**: Tokio + Axum Reflective Runtime microservice.

---

## 3. Mandatory CI Quality Verification Gates

Before declaring any task or feature complete, execute the full 4-step quality gate stack:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/run_pipeline.py --gate
python3 scripts/gen-llms.py
```

---

## 4. Master TODO & Future Roadmap

- [x] **Subprocess Failure Status Honesty**: Return `Err(OcrError::EngineError)` on process failure.
- [x] **Transitive Row-Bucket Sorting**: Group text blocks into vertical row buckets before Right-to-Left sorting.
- [x] **Art Region Overlap Avoidance**: Calculate `text_bounds.intersection_area_px(&art.bounds)`.
- [x] **Multilingual Cover Schema**: Model preserved English logos, credit metadata separation, and 9 cover text regions.
- [x] **Native C++ ONNX Engine Loading**: Wired direct C++ `Session` model loading (`from_onnx_file`) with in-memory sessions.
- [x] **Numerically Stable Softmax & Dynamic Confidence**: Implemented 2D tensor softmax $P(v)$ over vocab dimension and true geometric mean confidence $\exp(\frac{1}{N} \sum \ln(p_t))$.
- [x] **Five-Layer Semantic Validation Engine**: Built `ReadingOrderValidation` and `SemanticAssertion` modules in `comic-ocr-core::validation`.
- [x] **`15.png` RTL Traversal & Provenance Refinement**: Modeled 10 panels in strict Right-to-Left (RTL) reading order, decoupled OCR confidence from vision analysis confidence with explicit `AnalysisEvidence` provenance.
- [ ] **PDP Multi-Engine Consensus**: Calibrate Brier weights across local ONNX models and VLM APIs in `comic-ocr-pdp`.
- [ ] **Vector Contour Geometry Slicing**: Upgrade geometric detector from bounding boxes to exact polygonal speech-balloon contour masks.
