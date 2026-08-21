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
- [x] **Context Corpus Compiler Script (`scripts/gen-llms.py`)**: Generates `.agents/llms-cor.txt` and `.agents/llms-full-cor.txt`.
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
- [x] **Master Hierarchical Benchmark Ledger (`tests/data/benchmark_results.json`)**: Consolidated expected and actual OCR results across all 20 dataset images into a unified master ledger containing 5-level nested schema trees.
- [x] **`12.jpg` 2-Page Spread Topology Refinement**: Modeled 10 faithful panel regions (*koma*), natural localized translations, and SFX / non-balloon vocalization overlays (`グッ`, `フゥ`, `ム フフフ フウウ......`).
- [x] **`14.jpg` Multilingual Cover Art & Credit Metadata Separation**: Modeled 9 cover text regions (including preserved English logos `DRAGON QUEST`, `SERIES SEVEN`, `WARRIORS`, `1`), canonical page scene graph integration, and separate `credit` metadata (`type: "supervisor"`) cleanly decoupled from literal text translation.

---

## Phase 8: Native ONNX Execution, Softmax Normalization & Multi-Layer Validation Engine (Completed)

- [x] **Native C++ ONNX Runtime Engine Loading (`comic-ocr-ort`)**: Implemented direct C++ `Session` model loader (`from_onnx_file`, `from_onnx_bytes`) with $3 \times 224 \times 224$ RGB image tensor preprocessing and in-memory session execution (`Arc<Mutex<Session>>`).
- [x] **Numerically Stable Softmax & Geometric Confidence (`comic-ocr-ort`)**: Implemented 2D tensor softmax normalization $P(v)$ over the vocabulary dimension and true geometric mean confidence calculation $\exp(\frac{1}{N} \sum \ln(p_t))$.
- [x] **Python Subprocess Real Softmax Score Extraction (`comic-ocr-ort`)**: Updated inline Python script to extract real PyTorch token softmax probabilities (`torch.softmax(score[0], dim=-1)`) with zero hardcoded constants.
- [x] **Algorithmic Levenshtein CER Benchmark Assertions (`test_benchmark_dataset.rs`)**: Implemented `compute_cer(expected, actual)` and wired `OrtEngine::predict(&img)` directly into dynamic inference evaluation when unignored ($CER \le 0.20$ per item, $avg\_cer \le 0.05$).
- [x] **Five-Layer Semantic Validation Engine (`comic-ocr-core::validation`)**:
  - `ReadingOrderValidation`: Detects spatial panel sequence contradictions against declared reading direction (RTL or LTR).
  - `SemanticAssertion`: Validates semantic roles and detects `number-role-conflict` errors when a page number badge (e.g. `14`) is conflated with `chapter_number` when continuation text specifies `第2章へつづく`.
  - `AnalysisEvidence`: Tracks provenance across `ocr`, `vision`, `geometry`, `metadata`, `language_model`, `human`, and `fixture`.
- [x] **`15.png` Comprehensive Scene Graph & RTL Reading Order Correction**:
  - Modeled 10 panels in strict Right-to-Left (RTL) manga reading order.
  - Decoupled full-page raw OCR confidence (`0.3314`) from sub-region vision analysis (`0.9890`) with explicit `AnalysisEvidence` provenance.
  - Marked top header banner as `test-annotation` / `fixture` (`localizable: false`).
  - Parsed `冒険メモ 🐾` parchment into a structured quest log document hierarchy with progress trackers.
- [x] **Enforced All-Targets Clippy & GitHub Actions CI Gate (`.github/workflows/ci.yml`)**:
  - Resolved `needless_range_loop` warnings in test targets.
  - Upgraded GitHub CI workflow (`.github/workflows/ci.yml`) to run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` (87 passed, 2 ignored out of 89 total targets), and `cargo test -p comic-ocr-ort --test test_no_fabricated_output`.
- [x] **22-Image Local Benchmark Ledger (`tests/data/benchmark_results.json`)**:
  - Consolidated 22 local test dataset images into a unified master ledger. *(Note: Distinct from the platform's 830-page `panel-detector` staging run in `manga-service`).*
- [x] **ONNX Model Export Script & Graph Contract (`scripts/export_onnx.py`, `docs/ONNX_GRAPH_CONTRACT.md`)**:
  - Created [`docs/ONNX_GRAPH_CONTRACT.md`](file:///Users/zachshallbetter/Projects/comic-ocr-rust/docs/ONNX_GRAPH_CONTRACT.md) defining tensor contracts and KV-cache mechanics.
  - Created [`scripts/export_onnx.py`](file:///Users/zachshallbetter/Projects/comic-ocr-rust/scripts/export_onnx.py) to export `kha-white/manga-ocr` into `models/onnx/{encoder_model.onnx, decoder_model.onnx, decoder_with_past_model.onnx}`.

---

## Phase 9: Future Roadmap & Outstanding Engineering Directives (TODO / In Progress)

### A. Pure Rust `VisionEncoderDecoder` Generation Loop (Native ONNX Decoder)

- [ ] **Generate ONNX Model Graphs (`models/onnx/`)**:
  - Execute `python3 scripts/export_onnx.py --model kha-white/manga-ocr --output-dir models/onnx` to restore model graphs.
- [ ] **Empirically Verify Rust Decoder KV-Cache Loop (`crates/comic-ocr-ort/src/generate.rs`)**:
  - `generate.rs` is **Implemented (Unverified)**. Unignore `test_benchmark_model_inference_evaluation` in `test_benchmark_dataset.rs` and verify native Rust KV-cache generator against ONNX model weights once graphs are present.

### B. Persistent Python Model Inference Worker

- [ ] **Long-Lived Python Worker / Process Pool**:
  - Implement a persistent background Python daemon/worker process over stdin/stdout IPC or Unix domain socket for `OrtEngine` when operating in subprocess mode, eliminating per-image PyTorch/transformers model reloading overhead across batch operations.

### C. PDP Multi-Engine Consensus & Brier Calibration (`comic-ocr-pdp`)

- [ ] **Multi-Engine PDP Consensus Calibration**:
  - `comic-ocr-pdp` currently contains 70 lines of substrate types; Brier calibration is **Unbuilt / Design Intent**.
  - Implement Brier-calibrated consensus weighting $w_i = \exp(-\text{Brier}_i)$ across local ONNX model predictions and remote VLM API transcriptions (e.g., Gemini / Claude) in `comic-ocr-pdp`.
- [ ] **Cross-Engine Disagreement Detection**:
  - Flag region transcriptions with high CER disagreement for automated human-in-the-loop review.

### D. Advanced Cover & Editorial Typography Reconstruction

- [ ] **Vector Contour Geometry Slicing**: Upgrade geometric detector from coarse bounding boxes to exact polygonal speech-balloon and title contour masks.
- [ ] **Cover Font & Style Extraction**: Automatically infer color, stroke width, and gradient fill attributes from cover art text regions for fidelity-preserving re-lettering rendering.

### E. Distillation Exporter, Composed Confidence & Independent Reader Flywheel

- [ ] **Composed Pair Confidence Calculation**:
  - Implement $\mathbf{C}_{\text{pair}} = \mathbf{C}_{\text{detector}} \times \mathbf{C}_{\text{transcriber}}$ composed confidence calculation when creating training pairs from `IPubSemanticResource` envelopes.
- [x] **Training Pair Exporter CLI & Library (`export_pairs`)**:
  - `comic-ocr-core` writes real crops, canonical `training_pair.json` records,
    counted rejection telemetry, and `dataset_manifest.json`; the CLI requires
    canonical page/envelope ids, rights grant, class and split context.
  - `ExportFilter` selects one closed class (`silver`, `gold`, `evaluation`),
    confidence/language/crop bounds, and strictly excludes `rejected`.
- [ ] **Platform dataset orchestration**:
  - Resolve page and semantic-envelope bytes from CAS, validate an active
    `semantic_training_grant`, enforce corpus-wide crop deduplication and
    split-group isolation, then invoke the implemented exporter boundary.
- [ ] **Held-Out Human Evaluation Set Discipline**:
  - Reserve a strictly held-out, human-reviewed evaluation test corpus (e.g. `package 0000` / `test_benchmark_dataset.rs`) for measuring real model reading accuracy, strictly isolated from machine-labeled training signals.
- [ ] **Cross-Engine Disagreement Review Queue**:
  - Compare `comic-ocr-rust` predictions against 3rd-party teacher (Gemini `ocr-detector`) predictions to compute un-correlated cross-engine disagreement matrices and route disagreements directly to human review queues.
