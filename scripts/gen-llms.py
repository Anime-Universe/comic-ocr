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
import subprocess
from pathlib import Path

ROOT = Path(__file__).parent.parent.resolve()
AGENTS_DIR = ROOT / ".agents"
DOCS_DIR = ROOT / "docs"
CRATES_DIR = ROOT / "crates"

AGENTS_DIR.mkdir(parents=True, exist_ok=True)

# The manifest is DERIVED, never hand-written.
#
# It used to be a string constant listing five crates (there are six) and
# describing `comic-ocr-ort` as "token entropy loop truncation" with no mention
# of the generation loop — which is the head commit. An index that cannot drift
# toward the truth only drifts away from it, and the cost is real: on 2026-08-20
# a reader concluded from this file that the decoder loop did not exist while it
# sat, complete and tested, in `comic-ocr-ort/src/generate.rs`.
#
# So the index now answers "does X already exist?" from the tree itself. That is
# the question it is actually asked.


def _crate_doc(crate_dir: Path) -> str:
    """The crate's own one-line description, from its lib.rs //! header."""
    for candidate in ("src/lib.rs", "src/main.rs"):
        f = crate_dir / candidate
        if not f.exists():
            continue
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if line.startswith("//!") and line[3:].strip():
                return line[3:].strip()
            if line and not line.startswith("//"):
                break
    return ""


def _public_symbols(crate_dir: Path):
    """Public API per module — what an agent greps when asking 'is this built?'"""
    out = {}
    src = crate_dir / "src"
    if not src.exists():
        return out
    sym = re.compile(r"^\s*pub (?:async )?(fn|struct|enum|trait|const) ([A-Za-z_][A-Za-z0-9_]*)")
    for f in sorted(src.rglob("*.rs")):
        names = []
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            m = sym.match(line)
            if m:
                names.append(f"{m.group(2)}")
        if names:
            out[str(f.relative_to(crate_dir))] = names
    return out


def build_manifest() -> str:
    root = Path(ROOT_DIR) if "ROOT_DIR" in globals() else Path(__file__).resolve().parent.parent
    parts = ["# Comic OCR Rust Context Manifest (llms.txt)", ""]
    try:
        head = subprocess.check_output(["git", "log", "-1", "--format=%h %s"], cwd=root,
                                       stderr=subprocess.DEVNULL, text=True).strip()
        dirty = subprocess.check_output(["git", "status", "--porcelain"], cwd=root,
                                        stderr=subprocess.DEVNULL, text=True).strip()
        parts.append(f"> HEAD: {head}{'  [WORKING TREE DIRTY — this corpus is not any committed state]' if dirty else ''}")
    except Exception:
        parts.append("> HEAD: unknown (not a git checkout)")
    parts += ["> Generated from the tree. Do not hand-edit; edit the generator.", ""]

    docs = sorted((root / "docs").glob("*.md")) if (root / "docs").exists() else []
    if docs:
        parts.append(f"## Documents ({len(docs)})")
        for d in docs:
            first = ""
            for line in d.read_text(encoding="utf-8", errors="replace").splitlines():
                if line.strip() and not line.startswith("#"):
                    first = line.strip()[:110]
                    break
            parts.append(f"- [{d.name}](docs/{d.name}){': ' + first if first else ''}")
        parts.append("")

    crates_dir = root / "crates"
    crates = sorted([c for c in crates_dir.iterdir() if c.is_dir()]) if crates_dir.exists() else []
    parts.append(f"## Crates ({len(crates)})")
    parts.append("")
    for c in crates:
        doc = _crate_doc(c)
        parts.append(f"### `{c.name}`{' — ' + doc if doc else ''}")
        syms = _public_symbols(c)
        if not syms:
            parts.append("  (no public symbols found)")
        for mod, names in syms.items():
            shown = ", ".join(names[:14])
            more = f" (+{len(names) - 14} more)" if len(names) > 14 else ""
            parts.append(f"- `{mod}`: {shown}{more}")
        parts.append("")
    return "\n".join(parts) + "\n"


MANIFEST_HEADER = None  # replaced by build_manifest(); kept so old references fail loudly


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

    manifest_path.write_text(sanitize_content(build_manifest()), encoding="utf-8")

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
