# Comic OCR Rust: Implementation & Audit Master TODO

> Master implementation roadmap and completed audit checklist for **Comic OCR Rust**.

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
- [x] **Context Corpus Compiler Script (`scripts/gen-llms.py`)**: Generated `.agents/llms.txt` and `.agents/llms-full.txt` (128KB).
- [x] **Authoritative JSON Schema Suite (`schemas/`)**: Created `ocr_result.json`, `page_result.json`, `pdp_decision.json`, `comic_scene_graph.json`, and `localized_text_object.json`.

---

## Phase 5: 4-Layer Comic Scene Graph & Localization Engine (In Progress)

- [x] **Scene Graph Data Substrate (`comic-ocr-core`)**: Defined `MangaDocument`, `MangaPage`, `PanelBand`, `Panel`, `TextContainer`, `TextRegion`, `DualRect`, `LayoutEnvelope`, `ArtRegion`, `MaskRegion`.
- [ ] **Scene Graph Parser & Dual Coordinate Converter (`comic-ocr-core`)**: Implement JSON import/export parser and bi-directional pixel <-> normalized coordinate scaling (`px` $\leftrightarrow$ `normalized`).
- [ ] **Topological Panel Graph Extractor (`comic-ocr-core`)**: Extract panel bands, content bounds, safe bounds, bleed bounds, and explicit reading order sequence from page artwork.
- [ ] **Protected Art Region Avoidance Solver (`comic-ocr-core`)**: Segment character faces, eyes, and important artwork to evaluate `AvoidConstraint` penalty maps during layout placement.
- [ ] **Layout Envelope & Auto-Lettering Solver (`comic-ocr-core`)**: Fit localized text within `SpatialBounds` (`min`, `preferred`, `max`, `hard`) and `TypographyEnvelope` limits without overlapping protected art.
- [ ] **Background Cleanup Mask Generator (`comic-ocr-core`)**: Generate solid-fill balloon masks and inpaint texture masks for sound effect removal.
- [ ] **Reflective Runtime Scene Graph REST API (`comic-ocr-runtime`)**:
  - `POST /v1/scene/compile`: Compile full `MangaDocument` authoring scene graph into compact `LocalizedTextObject` runtime payloads.
  - `POST /v1/scene/validate`: Validate scene graphs against collision, overflow, face-obstruction, and reading order rules (`ValidationIssue`).
  - `POST /v1/scene/layout`: Solve auto-lettering text placement within container envelopes.

---

## Phase 6: Edge Deployment & Python Wheels (Next Steps)
- [ ] **PyO3 Zero-Copy Bindings (`comic-ocr-py`)**: Maturin C-extension module compiling Rust engine to Python wheel.
- [ ] **IEPE Parity Verification Gate**: Execute automated parity test comparing Rust ONNX outputs against PyTorch baselines.
