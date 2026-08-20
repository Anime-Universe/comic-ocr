# Comic OCR Rust: Implementation & Governance TODO Master

**Branch:** `rust-migration`  
**Last Updated:** `2026-08-19`  
**Status Legend:**  

- [x] **Completed & Verified** (Passing tests / committed)  
- [ ] **Pending Next Phase** (Planned per Master Architecture Specification)  

---

## Phase 1: Legacy Code Audit, Python Refactoring & Hardening

- [x] **Comprehensive Codebase Review**: Completed architecture audit (`docs/code_review.md`).
- [x] **Linux Wayland Clipboard Setter Fix**: Resolved `pyperclip` crash on Wayland sessions.
- [x] **Thread-Safety Hardening**: Removed `global OUT_DIR` state mutations in synthetic generator.
- [x] **Environment Variable Overrides**: Enabled `MANGA_OCR_DIR` environment overrides.
- [x] **Type Annotations**: Added static type hints and `from __future__ import annotations`.
- [x] **Hugging Face `evaluate` CER Metric Migration**: Migrated metric loader to `evaluate.load("cer")`.
- [x] **Repository Hardening**: Updated `.gitignore` to exclude ML logs, model weights, and OS files.
- [x] **Dependencies & Version Constraints**: Added `torchvision` to `pyproject.toml` and bound Python to `<3.13`.
- [x] **Image Crop Ingestion & Inference Testing**: Added `assets/examples/12.jpg`, `13.jpg`, `14.jpg` and ran inference over all 15 images.

---

## Phase 2: Feature Upgrades & Microservice Foundations

- [x] **Sequence Confidence Score Calculation**: Implemented $S = \exp(\frac{1}{N}\sum \ln P_i)$ geometric mean confidence scoring.
- [x] **Batch Inference API (`predict_batch`)**: Added matrix batch prediction for processing list of crops.
- [x] **Lightweight ONNX Runtime Backend**: Implemented `MangaOcrOnnx` and `export_to_onnx()` helper.
- [x] **Event-Driven File Watcher**: Added native `watchdog` observer with polling loop fallback.
- [x] **FastAPI REST API Microservice**: Built `/health`, `/ocr`, and `/ocr/batch` API server endpoints.
- [x] **Docker Containerization**: Created multi-stage `Dockerfile`.
- [x] **Full Page Processing Pipeline**: Implemented `MangaPagePipeline` with bounding-box cropping and reading-order sorting.

---

## Phase 3: Architecture & Governance Research Synthesis

- [x] **Framework & Doctrine Synthesis**: Unified Reflective Rust (RRSA), PDP, IEPE, Draft Smarter, and Titan blueprints (`docs/architecture_and_doctrine.md`).
- [x] **Reflective Rust Integration & Gains**: Documented RSP zero-copy PyO3 FFI and CSG static shape validation (`docs/reflective_rust_integration.md`).
- [x] **Polymorphic Decision Protocol (PDP)**: Documented 4-phase panel evaluation, ACS discounting, and Brier calibration (`docs/pdp_integration.md`).
- [x] **Intent & Evidence Project Engine (IEPE)**: Documented qualification trace, ticket-first discipline, and assertion-checked gates (`docs/iepe_integration.md`).
- [x] **Agent, Skill & Script Methods**: Documented agent orchestration, `.agents/skills` taxonomy, and `scripts/gen-llms.py` (`docs/agent_and_skill_methods.md`).
- [x] **Reference ComicOCR Benchmark Analysis**: Analyzed PaddleOCR/TrOCR reference project and 8MB model target (`docs/reference_mangaocr_learnings.md`).
- [x] **Master Architecture & Systems Specification**: Compiled canonical master technical specification incorporating all domain solutions (Furigana, Tate-chū-yoko, Sound Effects, aspect-ratio resampling, loop truncation, panel hierarchy, dual 8MB/430MB PDP escalation) (`docs/MASTER_ARCHITECTURE_SPECIFICATION.md`).

---

## Phase 4: Pure Rust Workspace Setup & Python Cleanup

- [x] **Branch Creation & Remote Push**: Created `rust-migration` branch and pushed to GitHub.
- [x] **Complete Python Codebase Removal**: Stripped 100% of legacy `.py` files (`comic_ocr`, `comic_ocr_dev`, `tests/*.py`, `.venv`).
- [x] **Cargo Workspace Manifest**: Created root `Cargo.toml` & `Cargo.lock` with Edition 2024 and MSRV 1.88.
- [x] **`crates/comic-ocr-core`**: Implemented `OcrEngine` trait, `OcrResult` struct, `EngineType` enum, and Japanese full-width `post_process()`.
- [x] **`crates/comic-ocr-pdp`**: Implemented `PanelEvaluator`, ACS discounting, and pre-committed invalidation triggers.
- [x] **`crates/comic-ocr-ort`**: Implemented `OrtEngine` C-bindings stub.
- [x] **`crates/comic-ocr-cli`**: Implemented high-performance CLI binary (`comic-ocr`).
- [x] **`crates/comic-ocr-runtime`**: Restructured service into Titan-style Reflective Runtime with Tokio, Axum, CORS, metrics, and graceful shutdown.
- [x] **Pure Rust Multi-Stage Dockerfile**: Created multi-stage Rust release build `Dockerfile`.
- [x] **Pure Rust GitHub Actions CI**: Updated `.github/workflows/main.yml` to run `cargo fmt`, `cargo clippy`, and `cargo test`.
- [x] **Workspace Testing**: Executed `cargo check` and `cargo test` (**3 passed in 2.37s**).

---

## Phase 5: Deep Crate Implementation & Model Integration

- [x] **ONNX C-API Engine Loading (`comic-ocr-ort`)**: Implemented `OrtEngine` with ONNX session management contracts and model selection flags.
- [x] **Dual Model Footprint Profile (`comic-ocr-nano`)**: Integrated `EngineType::NanoMobileNet` (~8MB footprint) and `EngineType::BaseInt8Onnx` profiles.
- [x] **Furigana Bracket Parser FSM (`comic-ocr-core`)**: Implemented 4-state FSM emitting `漢[かん]字[じ]` when `extract_furigana=true`.
- [x] **Aspect-Ratio Preserving Multi-Tile Resampling (`comic-ocr-core`)**: Implemented `resample_tiles` sliding window slicing ($\delta = 0.20$ overlap) for crops with aspect ratio $> 3:1$.
- [x] **Autoregressive Attention Loop Truncation (`comic-ocr-ort`)**: Implemented token entropy calculation $H_k$ and rolling entropy check ($\bar{H}_{k-3:k} < 0.15$) to terminate degenerate loops.
- [x] **2-Level Topological Panel Graph (`comic-ocr-core`)**: Implemented `sort_bubble_reading_order` for Right-to-Left, Top-to-Bottom speech bubble sorting.
- [ ] **PyO3 Zero-Copy Bindings (`comic-ocr-py`)**: Maturin C-extension module for compiling Rust engine to Python wheel.
- [x] **Context Corpus Compiler Script (`scripts/gen-llms.py`)**: Generated `.agents/llms.txt` manifest and `.agents/llms-full.txt` (123KB single-file context corpus).
- [ ] **IEPE Parity Verification Gate**: Execute automated parity test comparing Rust ONNX outputs against PyTorch baseline images (0% CER divergence).
