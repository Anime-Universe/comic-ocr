#!/usr/bin/env python3
"""Context Compiler Script for Comic OCR Rust.

Compiles repository markdown documentation, architecture specifications, API contracts,
and Rust crate definitions into single-file LLM context corpora:
  - .agents/llms.txt (Manifest & Summary)
  - .agents/llms-full.txt (Full Aggregated Context Corpus)

Ensures all generated context output is sanitized and free of absolute local filesystem paths.
"""

import os
import re
from pathlib import Path

ROOT = Path(__file__).parent.parent.resolve()
AGENTS_DIR = ROOT / ".agents"
DOCS_DIR = ROOT / "docs"
CRATES_DIR = ROOT / "crates"

AGENTS_DIR.mkdir(parents=True, exist_ok=True)

MANIFEST_HEADER = """# Comic OCR Rust Context Manifest (llms.txt)

> High-performance, zero-cost, multi-crate Rust workspace for optical character recognition of Japanese and English manga/comics.

## Workspace Architecture & System Modules

- [Master Architecture Specification](docs/MASTER_ARCHITECTURE_SPECIFICATION.md): Canonical technical specification.
- [API & Schema Reference](docs/api.md): Rust traits, JSON schemas, REST endpoints, CLI parameters.
- [Architecture & Doctrine Synthesis](docs/architecture_and_doctrine.md): PDP, IEPE, Reflective Rust, Titan blueprints.
- [Master TODO Ledger](docs/TODO.md): Implementation checklist & audit tracking.

## Core Crates Summary

- `comic-ocr-core`: Domain primitives, OcrEngine trait, multi-language post-processing (Japanese, English), Furigana FSM.
- `comic-ocr-pdp`: Polymorphic Decision Protocol panel evaluator & ACS discounting.
- `comic-ocr-ort`: ONNX Runtime (ort) C-bindings engine & token entropy loop truncation.
- `comic-ocr-cli`: Fast command-line binary (comic-ocr).
- `comic-ocr-runtime`: Titan-style Reflective Runtime microservice (Tokio + Axum).
"""


def sanitize_content(text: str) -> str:
    """Strips all absolute local filesystem paths (e.g. file:///Users/...)"""
    # Replace file:///Users/.../comic-ocr-rust/ with relative repo paths
    text = re.sub(r"file:///[^\s\)\"\']+/comic-ocr-rust/", "", text)
    # Replace any leftover file:///Users/... links
    text = re.sub(r"file:///[^\s\)\"\']+", "", text)
    # Replace raw user path strings like /Users/zachshallbetter/... with relative references
    text = re.sub(r"/Users/[a-zA-Z0-9_\-]+/Projects/comic-ocr-rust/", "", text)
    text = re.sub(r"/Users/[a-zA-Z0-9_\-]+/Projects/", "reference/", text)
    text = re.sub(r"/Users/[a-zA-Z0-9_\-]+/", "~/", text)
    return text


def compile_full_corpus():
    full_text_path = AGENTS_DIR / "llms-full.txt"
    manifest_path = AGENTS_DIR / "llms.txt"

    manifest_path.write_text(sanitize_content(MANIFEST_HEADER), encoding="utf-8")

    doc_files = sorted(list(DOCS_DIR.glob("*.md")))
    doc_files.insert(0, ROOT / "README.md")
    doc_files.insert(1, ROOT / "Cargo.toml")

    crate_sources = sorted(list(CRATES_DIR.glob("**/*.rs")))

    corpus_parts = []
    corpus_parts.append("================================================================================")
    corpus_parts.append("COMIC OCR RUST: AGGREGATED SINGLE-FILE CONTEXT CORPUS (llms-full.txt)")
    corpus_parts.append("================================================================================")
    corpus_parts.append("\n\n")

    for file_path in doc_files:
        if file_path.exists():
            rel_path = file_path.relative_to(ROOT)
            corpus_parts.append(f"\n\n--- FILE: {rel_path} ---\n\n")
            corpus_parts.append(sanitize_content(file_path.read_text(encoding="utf-8")))

    for src_path in crate_sources:
        rel_path = src_path.relative_to(ROOT)
        corpus_parts.append(f"\n\n--- RUST SOURCE: {rel_path} ---\n\n")
        corpus_parts.append(sanitize_content(src_path.read_text(encoding="utf-8")))

    full_corpus = "".join(corpus_parts)
    full_text_path.write_text(full_corpus, encoding="utf-8")

    size_kb = len(full_corpus.encode("utf-8")) / 1024.0
    print(f"[SUCCESS] Compiled context manifest: {manifest_path.relative_to(ROOT)}")
    print(f"[SUCCESS] Compiled full context corpus: {full_text_path.relative_to(ROOT)} ({size_kb:.2f} KB)")


if __name__ == "__main__":
    compile_full_corpus()
