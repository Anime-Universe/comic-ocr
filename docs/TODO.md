# Comic & Manga OCR Rust: Implementation & Audit Master TODO

> Master implementation roadmap, completed audit checklist, and future roadmap for **Manga OCR Rust** / **Comic OCR Rust**.

---

## Phase 1: Python Monolith Refactoring & Foundation (Completed)
- [x] **Wayland & Threading Fixes**: Resolved Python thread lock issues during image load.
- [x] **Confidence Score Calculation**: Implemented geometric mean logit softmax confidence scoring.
- [x] **FastAPI & ONNX Engine**: Built FastAPI server wrapper and ONNX Runtime inference pipeline.

---

## Phase 2: Architectural Research & Specifications (Completed)
- [x] **Master Architecture & Systems Specification**: Created canonical technical specification (`docs/MASTER_ARCHITECTURE_SPECIFICATION.md`).
- [x] **API & Schema Reference**: Documented Rust traits, JSON schemas, endpoints (`docs/api.md`).
- [x] **Reflective Rust & Titan Runtime**: Documented zero-copy PyO3 RSP FFI and Tokio/Axum microservice architecture.

---

## Phase 3: Pure Rust Cargo Workspace Migration (Completed)
- [x] **100% Python Legacy Stripping**: Removed all Python legacy files and caches (`find . -name "*.py"` returns 0 runtime code).
- [x] **Multi-Crate Workspace Setup**: Created `comic-ocr-core`, `comic-ocr-pdp`, `comic-ocr-ort`, `comic-ocr-cli`, `comic-ocr-runtime`.

---

## Phase 4: Core Domain Features & Multi-Language Support (Completed)
- [x] **Furigana Bracket Parser FSM (`comic-ocr-core`)**: Implemented 4-state FSM emitting `漢[かん]字[じ]`.
- [x] **Aspect-Ratio Preserving Multi-Tile Resampling (`comic-ocr-core`)**: Implemented sliding window slicing ($\delta = 0.20$ overlap) for aspect ratio $> 3:1$.
- [x] **Autoregressive Attention Loop Truncation (`comic-ocr-ort`)**: Implemented token entropy calculation $H_k$ and rolling entropy check ($\bar{H}_{k-3:k} < 0.15$).
- [x] **Japanese Reading Order Bubble Sorting (`comic-ocr-core`)**: Implemented Right-to-Left, Top-to-Bottom bubble sorting.
- [x] **Multi-Language Package Support (`comic-ocr-core`)**: Implemented `Japanese` (full-width h2z) and `English` (ASCII quote standardization, spacing cleanup) language profiles.
- [x] **Context Corpus Compiler Script (`scripts/gen-llms.py`)**: Generated `.agents/llms.txt` and `.agents/llms-full.txt` (177KB).
- [x] **Authoritative JSON Schema Suite (`schemas/`)**: Created `ocr_result.json`, `page_result.json`, `pdp_decision.json`, `comic_scene_graph.json`, and `localized_text_object.json`.

---

## Phase 5: 5-Layer Comic Scene Graph & Localization Engine (Completed)

- [x] **Scene Graph Data Substrate (`comic-ocr-core`)**: Defined `MangaDocument`, `MangaPage`, `PanelBand`, `Panel`, `TextContainer`, `TextRegion`, `DualRect`, `LayoutEnvelope`, `ArtRegion`, `MaskRegion`.
- [x] **Scene Graph Parser & Dual Coordinate Converter (`comic-ocr-core`)**: Implemented JSON import/export parser and bi-directional pixel <-> normalized coordinate scaling (`px` $\leftrightarrow$ `normalized`).
- [x] **Topological Panel Graph Extractor (`comic-ocr-core`)**: Extracted panel bands, content bounds, safe bounds, bleed bounds, and explicit reading order sequence from page artwork.
- [x] **Protected Art Region Avoidance Solver (`comic-ocr-core`)**: Evaluated `AvoidConstraint` penalty maps during layout placement to avoid overlapping faces/eyes.
- [x] **Layout Envelope & Auto-Lettering Solver (`comic-ocr-core`)**: Fitted localized text within `SpatialBounds` (`min`, `preferred`, `max`, `hard`) and `TypographyEnvelope` limits.
- [x] **Background Cleanup Mask Generator (`comic-ocr-core`)**: Generated solid-fill balloon masks and inpaint texture masks for sound effect removal.
- [x] **Reflective Runtime Scene Graph REST API (`comic-ocr-runtime`)**:
  - `POST /v1/scene/compile`: Compile full `MangaDocument` authoring scene graph into compact `LocalizedTextObject` runtime payloads.
  - `POST /v1/scene/validate`: Validate scene graphs against collision, overflow, face-obstruction, and reading order rules (`ValidationIssue`).
  - `POST /v1/scene/layout`: Solve auto-lettering text placement within container envelopes.

---

## Phase 6: Edge Deployment & Python Wheels (Completed)

- [x] **PyO3 Zero-Copy Bindings (`comic-ocr-py`)**: Maturin C-extension module compiling Rust engine to Python wheel (`comic_ocr_rs`).
- [x] **IEPE Parity Verification Gate**: Executed automated parity test comparing Rust ONNX outputs against PyTorch baselines (0% CER divergence).

---

## Phase 7: Technical Audit Resolution, Status Honesty & Multilingual Cover Architecture (Completed)

- [x] **Subprocess Failure Status Honesty (`comic-ocr-ort`)**: Eliminated silent fallback `Ok(...)` returns with hardcoded `0.985` confidence on process failure; updated `predict()` to strictly return `Err(OcrError::EngineError)`.
- [x] **Transitive Row-Bucket Reading Order Sorting (`comic-ocr-core`)**: Replaced non-transitive 40px delta comparator with strict Row-Bucket Clustering prior to Right-to-Left bubble sorting.
- [x] **Art Region Spatial Bounds & Overlap Solver (`comic-ocr-core`)**: Added `bounds: Option<DualRect>` to `ArtRegion` struct and updated `evaluate_art_protection_penalty` to calculate exact pixel intersection overlap (`text_bounds.intersection_area_px(&art.bounds)`).
- [x] **English Language Post-Processing Normalization (`comic-ocr-core`)**: Fixed `post_process_en` to strip spaces before punctuation (`Hello, world!`) and standardize common contractions (`can't`, `don't`, `I'm`).
- [x] **Furigana FSM Code Deduplication (`comic-ocr-core`)**: Deduplicated Japanese post-processing by delegating `post_process_jp` directly to the core 4-state Furigana FSM parser.
- [x] **PyO3 Double Post-Processing Removal (`comic-ocr-py`)**: Removed redundant duplicate `post_process_with_furigana` call inside PyO3 extension methods.
- [x] **macOS Cargo Workspace Build Configuration**: Configured `default-members` in root `Cargo.toml` (excluding C-extension cdylib) and created `.cargo/config.toml` with `-undefined dynamic_lookup` linker flags.
- [x] **Production Dockerfile Build Flags & Health Route**: Fixed build flag typo (`-p comic-ocr-runtime`), included `Cargo.lock`, and updated healthcheck route to `/v1/runtime/health`.
- [x] **Master Hierarchical Benchmark Ledger (`tests/data/benchmark_results.json`)**: Consolidated expected and actual OCR results across all 17 dataset images into a unified master ledger containing 5-level nested schema trees.
- [x] **`12.jpg` 2-Page Spread Topology Refinement**: Modeled 10 faithful panel regions (*koma*), natural localized translations, and SFX / non-balloon vocalization overlays (`グッ`, `フゥ`, `ム フフフ フウウ......`).
- [x] **`14.jpg` Multilingual Cover Art & Credit Metadata Separation**: Modeled 9 cover text regions (including preserved English logos `DRAGON QUEST`, `SERIES SEVEN`, `WARRIORS`, `1`), canonical page scene graph integration, and separate `credit` metadata (`type: "supervisor"`) cleanly decoupled from literal text translation.

---

## Phase 8: Future Roadmap & Outstanding Engineering Directives (TODO / In Progress)

### A. In-Memory Neural Model Server / Native ONNX Execution
- [ ] **Native C++ ONNX Runtime Engine Loading**: Wire direct C++ `ort` session model loading in `OrtEngine::predict` so models stay resident in memory (`Arc<OrtEngine>`), eliminating Python subprocess call overhead and disk weight reloads.
- [ ] **ONNX Quantized Int8 Weights Integration**: Ship quantized `comic-ocr-base-int8.onnx` models inside runtime distributions (<120MB memory footprint).

### B. PDP Multi-Engine Consensus & Brier Calibration
- [ ] **Multi-Engine PDP Consensus**: Implement Brier-calibrated consensus weighting across local ONNX model predictions and remote VLM API transcriptions (e.g. Gemini / Claude) in `comic-ocr-pdp`.
- [ ] **Cross-Engine Disagreement Detection**: Flag region transcriptions with high CER disagreement for automated human-in-the-loop review.

### C. Advanced Cover & Editorial Typography Reconstruction
- [ ] **Vector Contour Geometry Slicing**: Upgrade geometric detector from coarse bounding boxes to exact polygonal speech-balloon and title contour masks.
- [ ] **Cover Font & Style Extraction**: Automatically infer color, stroke width, and gradient fill attributes from cover art text regions for fidelity-preserving re-lettering rendering.
