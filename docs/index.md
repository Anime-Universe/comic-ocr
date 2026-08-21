# Comic OCR Rust Documentation Index

Welcome to the technical documentation for **Comic OCR Rust** (`comic-ocr-rust`). This repository contains the zero-cost, multi-crate Rust workspace for high-performance optical character recognition of Japanese manga.

---

## Workspace Architecture

Comic OCR Rust is organized as a production-grade Cargo workspace:

```mermaid
flowchart TD
    CLI["comic-ocr-cli (CLI Binary)"] --> CORE["comic-ocr-core (Domain Primitives)"]
    RUNTIME["comic-ocr-runtime (Tokio/Axum Service)"] --> CORE
    RUNTIME --> PDP["comic-ocr-pdp (PDP Evaluator)"]
    ORT["comic-ocr-ort (ONNX C-Bindings Engine)"] --> CORE
    PDP --> ORT
```

- **`comic-ocr-core`**: Core domain types, `OcrEngine` trait, Japanese full-width post-processing.
- **`comic-ocr-pdp`**: Polymorphic Decision Protocol panel evaluator & ACS consensus discounting.
- **`comic-ocr-ort`**: C++ ONNX Runtime bindings (`ort`), tensor memory, greedy/beam decoding.
- **`comic-ocr-cli`**: High-performance command-line binary (`comic-ocr`).
- **`comic-ocr-runtime`**: Titan-style Reflective Runtime microservice (Tokio + Axum).

---

## Repository Documentation Map

- [**Master TODO & Implementation Ledger**](docs/TODO.md): Full audit checklist tracking completed tasks and next phase implementation goals.
- [**Master Architecture & Systems Specification**](docs/MASTER_ARCHITECTURE_SPECIFICATION.md): Master technical specification unifying system evolution, Cargo workspace blueprints, PDP/IEPE doctrines, RRSA integration, theoretical solutions, findings, and migration roadmap.
- [**API & Microservice Reference**](docs/api.md): Detailed reference for Rust library traits, JSON schemas, Reflective Runtime REST endpoints, CLI parameters, and Docker deployment.
- [**Architecture & Doctrine Synthesis**](docs/architecture_and_doctrine.md): Unifies Reflective Rust (RRSA), Polymorphic Decision Protocol (PDP), IEPE intent-evidence loops, Draft Smarter scoring/probabilities, and Titan production Rust runtime blueprints.
- [**Reflective Rust Integration & Gains**](docs/reflective_rust_integration.md): Deep dive into RRSA integration, zero-overhead PyO3 FFI, compile-time tensor shape checks, and CSG model self-description.
- [**PDP Integration & Gains**](docs/pdp_integration.md): Details Polymorphic Decision Protocol integration, ACS consensus discounting, Brier calibration, and invalidation triggers.
- [**IEPE Governance Integration & Gains**](docs/iepe_integration.md): Details Intent and Evidence Project Engine qualification trace, ticket-first discipline, and verification gates.
- [**Agent, Skill & Automation Methods**](docs/agent_and_skill_methods.md): Details agent orchestration, `.agents/skills` taxonomy, and `scripts/gen-llms.py` context compilation.
- [**Reference ComicOCR Analysis & Learnings**](docs/reference_mangaocr_learnings.md): Analysis of PaddleOCR/TrOCR reference project, ~8MB model size target, and long-sequence attention bug mitigations.
- [**Page Processing Strategy**](docs/page_processing_strategy.md): Architecture map and strategies for full-page OCR, color cover handling, and cross-panel text bubbles.
- [**Ingestion Loop Contract**](docs/ingestion_contract.md): Six-stage discover/decode/segment/normalize/recognize/emit contract binding the CLI, pipeline script, and REST runtime, with invariants, error classes, conformance tests, and current gaps.
- [**iPub Format & Infinite Verse Integration Specification**](docs/ipub_format_and_infinite_verse_integration.md): Comprehensive specification of the iPub format, CAS asset manifests, page-semantics envelope schemas, database attachment rules, and Mode A/Mode B integration seams.
- [**Flywheel, Distillation & Independent Reader Architectural Doctrine**](docs/FLYWHEEL_DISTILLATION_ARCHITECTURAL_DOCTRINE.md): Core architectural doctrine detailing VisionEncoderDecoder model scope, composed pair confidence, distillation error-correction, the independent reader principle, and held-out human evaluation set rules.
- [**Rust Migration Specification**](docs/rust_migration.md): Technical specifications, blueprints, and verification gates for the Inference Bridge, PyO3 Python wheel, and Neural Model & PDP Architecture.
- [**JSON Schema Suite & 12.jpg Execution Reference**](docs/json_schema_suite.md): Details all 7 canonical JSON Schema contracts, sample instances, and multi-schema execution results.
- [**Training Pair Export Specification**](TRAINING_EXPORT.md): Documents pair generation, silver/gold/evaluation isolation, rights grants, immutable manifests, and The Training Contract rules.
- [**Code Review & Audit Report**](docs/code_review.md): Deep-dive audit report highlighting architectural strengths, known gaps, code smells, test coverage gaps, and refactoring roadmap.

---

## Technology Stack

- **Language**: Rust (Edition 2024, MSRV 1.88)
- **ONNX Runtime**: [`ort`](https://crates.io/crates/ort) v2.0.0 (C++ dynamic binding)
- **Async Runtime & Web Service**: `tokio` v1.38, `axum` v0.7, `tower-http` v0.5
- **CLI & Diagnostics**: `clap` v4.5, `tracing` v0.1, `tracing-subscriber` v0.3
- **Data Primitives**: `serde`, `serde_json`, `image` v0.25, `thiserror`, `anyhow`
