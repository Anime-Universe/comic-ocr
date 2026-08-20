# Manga & Comic OCR Rust

High-performance, zero-cost, multi-crate Rust workspace for optical character recognition of **Manga, Western Comics, and Graphic Novels** in both **Japanese** and **English**.

Paged formats. Long-strip webtoons are out of scope for v1 — they are a different page geometry, not just another language, and the corpus this serves contains none (0 of 1,019 pages are strip-shaped).

---

## Content & Format Scope

- **Japanese Manga & Manhua**: Vertical reading mode (`vertical-rl`), horizontal text, Furigana reading extraction (`漢[かん]字[じ]`), Tate-chū-yoko patch rotation, dynamic sound effect (*onomatopoeia*) LM bypass.
- **English Comics & Graphic Novels**: Western reading order, speech bubble text, ASCII punctuation normalization, contraction standardization, and clean formatting.
- **Tall text regions**: Multi-tile aspect-ratio preserving sliding window resampling ($\delta = 0.20$ overlap) for text regions too tall to survive a square encoder input — principally vertical Japanese columns. *Wired in CLI pipeline; see `resample.rs`.*
- **Full Page & Speech Bubble Topology**: 2-Level topological reading order graph sorting speech bubbles Right-to-Left / Left-to-Right and Top-to-Bottom.

---

## Workspace Crates & Packages

- **[`comic-ocr-core`](crates/comic-ocr-core)**: Pure Rust domain primitives, tokenization, multi-language post-processing (`Japanese`, `English`), Furigana FSM, multi-tile resampling, reading order layout sorting, and `OcrEngine` trait.
- **[`comic-ocr-pdp`](crates/comic-ocr-pdp)**: Polymorphic Decision Protocol substrate types for multi-engine panel evaluation (Brier calibration in progress).
- **[`comic-ocr-ort`](crates/comic-ocr-ort)**: C++ ONNX Runtime bindings (`ort`) managing tensor memory, image resizing, token entropy calculation ($H_k$), and loop truncation (<120MB RAM).
- **[`comic-ocr-cli`](crates/comic-ocr-cli)**: Fast, native command-line binary (`comic-ocr`).
- **[`comic-ocr-runtime`](crates/comic-ocr-runtime)**: High-throughput Tokio/Axum REST Reflective Runtime microservice.

---

## Runtime Engine Modes & Environment Requirements

- **Current Neural Model Inference Path**: Out-of-the-box neural recognition uses PyTorch/HuggingFace transformers (`python3`, `torch`, `transformers`) executed via `OrtEngine` with zero-copy stream processing and per-token logit softmax calculation.
- **Native Rust Decoder Loop (In Active Development)**: `OrtEngine::from_onnx_file` provides C++ ONNX session loading. Full native Rust ViT Encoder + Autoregressive Transformer Decoder KV-cache generation loop (`generate.rs`) is tracked under Phase 9.

---

## Quick Start

### Build Workspace

```bash
cargo build --release
```

### Run Tests

```bash
cargo test --workspace
```

### Run CLI

```bash
cargo run --release -p comic-ocr-cli -- --image assets/examples/00.jpg --extract-furigana
```

### Run Tokio/Axum Reflective Runtime

```bash
cargo run --release -p comic-ocr-runtime
```

---

## Documentation

Full architectural specifications, research doctrines, and API contracts:

- [**Master Architecture & Systems Specification**](docs/MASTER_ARCHITECTURE_SPECIFICATION.md)
- [**Rust Migration Specification**](docs/rust_migration.md)
- [**ONNX Graph Contract**](docs/ONNX_GRAPH_CONTRACT.md)
- [**API & Schema Reference**](docs/api.md)
- [**Architecture & Doctrine Synthesis**](docs/architecture_and_doctrine.md)
- [**Reflective Rust Integration & Gains**](docs/reflective_rust_integration.md)
- [**PDP Integration & Gains**](docs/pdp_integration.md)
- [**IEPE Governance Integration & Gains**](docs/iepe_integration.md)
- [**Master TODO & Implementation Ledger**](docs/TODO.md)
