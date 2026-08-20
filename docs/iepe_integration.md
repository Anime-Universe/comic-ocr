# Intent & Evidence Project Engine (IEPE) Integration & Gain Analysis

This document details how the **Intent and Evidence Project Engine (IEPE)** operates, how it integrates into the **Comic OCR** Rust migration (`comic-ocr-rust`), and the concrete gains achieved.

---

## 1. How IEPE Operates: The Governance Loop

IEPE governs software engineering and architecture through a rigorous, evidence-backed qualification trace:

$$\text{Intent} \longrightarrow \text{Epic} \longrightarrow \text{Issue} \longrightarrow \text{Artifact} \longrightarrow \text{Evidence} \longrightarrow \text{Qualification} \longrightarrow \text{Promotion}$$

```mermaid
flowchart LR
    A[Project Intent] --> B[Epic Definition]
    B --> C[Ticket-First Issue Contract]
    C --> D[Artifact / Code Implementation]
    D --> E[Empirical Evidence Generation]
    E --> F[Qualification Gate]
    F --> G[Production Promotion]
```

### Core Principles of IEPE

1. **Ticket-First Rule**: No code is committed without an authorized issue contract specifying explicit acceptance criteria, evidence requirements, constraints, and stop conditions.
2. **Strict Maturity Disentanglement**:
   - **Work Status**: Backlog $\to$ Ready $\to$ In Progress $\to$ In Review $\to$ Done $\to$ Verified.
   - **Claim Taxonomy**: Documented $\neq$ Implemented $\neq$ Tested $\neq$ Empirically Validated.
   - **Maturity Lifecycle**: Intent $\to$ Explored $\to$ Specified $\to$ Prototyped $\to$ Implemented $\to$ Observed $\to$ Validated.
3. **Gates That Cannot Fail**: CI test runners and benchmark suites must report exact executed assertion counts to prevent silent skips or false green checkmarks.
4. **Domain-Neutral Core**: Core Rust libraries remain domain-neutral, isolating environment-specific code (PyO3 bindings, web server routes) inside dedicated adapters.

---

## 2. Integration into Comic OCR Rust Migration

```text
comic-ocr-rust/
├── .agents/                    # IEPE adoption profile & issue contracts
├── crates/
│   ├── comic-ocr-core/         # Domain-neutral core (post-processing & tokenizer)
│   ├── comic-ocr-ort/          # ONNX Runtime inference adapter
│   ├── comic-ocr-py/           # PyO3 Python binding adapter
│   └── comic-ocr-server/       # Tokio + Axum microservice adapter
```

### Adoption Trace for Rust Migration
- **Epic 1**: Pure Rust Post-Processing & Tokenizer Core (`comic-ocr-core`)
- **Epic 2**: ONNX Runtime Engine & Tensor Buffer Management (`comic-ocr-ort`)
- **Epic 3**: Zero-Copy PyO3 Python Interoperability (`comic-ocr-py`)
- **Epic 4**: High-Throughput Tokio/Axum REST API Microservice (`comic-ocr-server`)

---

## 3. Concrete Technical & Operational Gains

### Gain 1: Elimination of Silent Regressions & Drift
- **Problem**: In complex ML refactoring, PyO3 bindings or Rust ports often introduce subtle string encoding differences (e.g. Unicode full-width conversion discrepancies).
- **IEPE Fix**: Before any Rust crate feature is promoted, IEPE qualification gates execute strict parity tests against PyTorch outputs across the 15 `assets/examples/` images, ensuring **0% divergence**.

### Gain 2: Ticket-First Discipline & Bounded Authority
- **Integration**: Every pull request or feature commit references an explicit issue contract with pre-registered stop conditions and resource budgets.
- **Concrete Benefit**: Prevents scope creep, unreviewed architecture mutations, and undocumented API breaking changes.

### Gain 3: Verification Gates That Cannot Fail
- **Integration**: CI workflows report total assertions executed during `cargo test` and `pytest`.
- **Concrete Benefit**: Ensures zero false-positive green checkmarks in CI when tests are accidentally skipped or category-filtered.

### Gain 4: Modular & Domain-Neutral Architecture
- **Integration**: Decouples the core Japanese text normalization engine (`comic-ocr-core`) from Python runtime dependencies.
- **Concrete Benefit**: Enables using `comic-ocr-core` inside native Rust desktop apps, mobile devices (iOS/Android via C-ABI), CLI tools, or web servers with zero Python runtime dependency.

---

## 4. Summary Matrix

| Metric / Dimension | Standard Ungoverned Refactoring | IEPE Governed Rust Migration |
| :--- | :--- | :--- |
| **Commit Qualification** | Ad-hoc manual approval | **Strict evidence-based qualification trace** |
| **Parity Verification** | Spot checking | **Automated parity verification against PyTorch** |
| **CI Reliability** | Risk of false greens on skipped tests | **Assertions-checked verification gates** |
| **Code Modularization** | Tightly coupled Python bindings | **Domain-neutral Rust core + isolated adapters** |
| **Maturity Transparency** | Conflated "done" state | **Disentangled 3-axis maturity tracking** |
