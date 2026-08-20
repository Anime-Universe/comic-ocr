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
2. **Separation of Translation vs. Role Metadata**:
   - Keep text translation strictly faithful to the source characters.
   - Do **NOT** fold inferred credit roles (e.g. `"Supervisor: Yuji Horii"`) into `translation.localized`. Use separate `credit` metadata objects (`credit: { type: "supervisor", labelSource: "adjacent-english-text" }`).
3. **Multilingual Cover Art & Preserved Regions**:
   - Non-translated text on manga covers (e.g. English logos `DRAGON QUEST`, volume numbers `1`, tags `WARRIORS`) are first-class `text_regions` with `strategy: "preserve"` and `language: "en"`.
4. **Canonical Scene Graph Integration**:
   - `comic_scene_graph` is the authoritative source of truth. All `text_regions`, `containers`, and `panels` must be fully populated inside `comic_scene_graph.pages[0]`.

---

## 2. Core Workspace Crates

- **`comic-ocr-core`**: Domain primitives, `OcrEngine` trait, Furigana bracket FSM, multi-tile resampling, row-bucket reading order sorting, protected art avoidance solver, auto-lettering solver.
- **`comic-ocr-pdp`**: Multi-engine consensus evaluation, Brier-calibrated candidate selection, ACS score discounting.
- **`comic-ocr-ort`**: Neural inference bindings, Shannon token entropy ($H_k$), rolling loop truncation check ($\bar{H} < 0.15$).
- **`comic-ocr-cli`**: High-performance operational CLI (`comic-ocr`).
- **`comic-ocr-runtime`**: Tokio + Axum Reflective Runtime microservice.

---

## 3. Verification Gates

Before declaring any task or feature complete, execute:

```bash
cargo check --workspace
cargo test --workspace
python3 scripts/gen-llms.py
```

---

## 4. Master TODO & Future Roadmap

- [x] **Subprocess Failure Status Honesty**: Return `Err(OcrError::EngineError)` on process failure.
- [x] **Transitive Row-Bucket Sorting**: Group text blocks into vertical row buckets before Right-to-Left sorting.
- [x] **Art Region Overlap Avoidance**: Calculate `text_bounds.intersection_area_px(&art.bounds)`.
- [x] **Multilingual Cover Schema**: Model preserved English logos, credit metadata separation, and 9 cover text regions.
- [ ] **Native C++ ONNX Engine Loading**: Wire direct C++ `ort` session loading into `OrtEngine` so weights stay in memory.
- [ ] **PDP Multi-Engine Consensus**: Calibrate Brier weights across local ONNX models and VLM APIs in `comic-ocr-pdp`.
- [ ] **Vector Contour Geometry Slicing**: Upgrade geometric detector from bounding boxes to exact polygonal speech-balloon contour masks.
