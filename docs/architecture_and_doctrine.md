# Architecture & Doctrine Synthesis: Rust Migration (`comic-ocr-rust`)

This document unifies the architectural blueprints, governance protocols, mathematical scoring models, and Rust runtime conventions established across our reference ecosystem (**Reflective Rust**, **PDP**, **IEPE**, **Draft Smarter**, and **Titan**).

---

## 1. Governance & Boundary Invariants

### 1.1 Core Immutable Boundary
> **"The systems supply evidence and argument. The human owns the weights, the stakes, and the exit."**

All automated OCR pipelines, bounding-box candidate proposals, beam search decodings, and model confidence scores serve strictly as **evidentiary arguments**. The final decision to commit text annotations or accept OCR outputs remains under human/application ownership.

### 1.2 The Four-Tier Claim Taxonomy
Always maintain status honesty across code, tests, and documentation:

$$\text{Documented} \neq \text{Implemented} \neq \text{Tested} \neq \text{Empirically Validated}$$

- **Documented**: Protocol specs, architecture markdown documents, RFCs.
- **Implemented**: Concrete Rust crates, ONNX Runtime wrappers, PyO3 bindings, or DDL schemas.
- **Tested**: Passing unit, integration, and benchmark test suites in CI (`cargo test`, `pytest`).
- **Empirically Validated**: Measured real-world Japanese manga OCR accuracy (CER/WER) on external benchmark datasets.

### 1.3 Strict Authority Hierarchy
When specifications, Rust code, generated bindings, or AI outputs conflict, resolve in strict canonical order:

$$\text{Architecture & Protocol Specs} \to \text{Domain Schemas / Contracts} \to \text{Core Rust Engine} \to \text{PyO3 / API Adapters} \to \text{Generated Artifacts}$$

---

## 2. IEPE Intent & Evidence Execution Loop

Every step of our Rust migration follows the IEPE qualification trace:

$$\text{Intent} \longrightarrow \text{Epic} \longrightarrow \text{Issue} \longrightarrow \text{Artifact} \longrightarrow \text{Evidence} \longrightarrow \text{Qualification} \longrightarrow \text{Promotion}$$

- **Zero Silent Mutations**: Code or generated artifacts never silently alter protocol contracts.
- **Verification Gates That Cannot Fail**: CI gates must report exact executed assertion counts to prevent false green checkmarks on skipped tests.
- **Domain-Neutral Core Engine**: The core Rust crate (`comic-ocr-core`) remains domain-neutral and decoupled from I/O, UI frameworks, or specific Python runtimes.

---

## 3. Titan Production Rust Runtime Blueprint

Drawing directly from our production **Titan** runtimes (`vision-runtime`, `text-runtime`, `image-runtime`):

```text
comic-ocr-rust/
├── Cargo.toml                      # Workspace manifest (Rust 2024 edition, MSRV 1.88)
├── crates/
│   ├── comic-ocr-core/             # Zero-dependency domain types, post-processing, CER metrics
│   ├── comic-ocr-ort/              # ONNX Runtime (ort) inference engine (<150MB RAM)
│   ├── comic-ocr-py/               # PyO3 bindings exposing Rust speed to Python
│   └── comic-ocr-server/           # Async Tokio + Axum REST/gRPC OCR microservice
```

### Titan Engine Best Practices:
1. **Async Engine**: Built on `tokio` (v1.x) and `axum` (v0.7) for non-blocking multi-threaded request processing.
2. **Error Safety**: Strong typing with `thiserror` for library crates and `anyhow::Result` for runtime binaries.
3. **Structured Tracing**: `tracing` + `tracing-subscriber` with `JSON` output support.
4. **Zero-Copy Serialization**: `serde` and `serde_json` for high-throughput payload handling.

---

## 4. Probabilistic Scoring & Evidentiary Discounting (Draft Smarter Engine)

Inspired by the probabilistic modeling in **Draft Smarter** (`scoring.ts`, `icp.ts`, `evidence.ts`):

### 4.1 Sequence Confidence Score
$$S = \exp\left( \frac{1}{N} \sum_{i=1}^N \ln P(w_i \mid w_{<i}, \mathbf{X}) \right) \in [0.0, 1.0]$$

Geometric mean of sequence token probabilities derived from model logit softmax distributions.

### 4.2 Evidence Class Discounting (ACS)
In multi-engine panel evaluation (e.g. PyTorch `the reference checkpoint` + ONNX Runtime + Tesseract fallback):

$$W_{\text{final}} = W_{\text{raw}} \cdot \alpha_{\text{provenance}} \cdot \beta_{\text{consensus}}$$

Where:
- $\alpha_{\text{provenance}}$ discounts lower-quality OCR engines or noisy inputs.
- $\beta_{\text{consensus}}$ discounts vendor-correlated model predictions.

---

## 5. Reflective Rust (RRSA) Compiler & Semantic Graph (CSG)

Following **Reflective Rust**:
- **Phase-Staged Introspection**: Rust types expose consteval `core::meta::Info` semantic handles for zero-cost static reflection.
- **Runtime Semantic Projection**: Opt-in runtime type descriptors (`TypeDescriptor`) allowing dynamic Python / C API inspection without compromising memory safety.
