# Agent, Skill & Automation Script Methods

This document details the agent orchestration methods, skill taxonomy, workspace customization structure (`.agents/`), and automated context compilation scripts (`scripts/gen-llms.py`) derived from **IEPE** and adopted into **Comic OCR** (`comic-ocr-rust`).

---

## 1. Agent Architecture & Customization Root (`.agents/`)

IEPE establishes a standardized, file-system-driven agent environment rooted in `.agents/`:

```text
comic-ocr-rust/
├── .agents/
│   ├── AGENTS.md               # Workspace Agent Operating Rules & Authority Hierarchy
│   ├── llms.txt                # ~15 KB Agent Context Summary Manifest
│   ├── llms-full.txt           # ~350+ KB Single-File Consolidated Workspace Corpus
│   └── skills/                 # Custom Agent Skills
│       └── comic-ocr-expert/   # Master Domain, Verification Gate & Architectural Doctrine Skill
└── scripts/
    └── gen-llms.py             # Single-File Context Corpus Generator Script
```

---

## 2. The Skill Taxonomy & Lifecycle

Skills represent specialized instruction manuals stored as `.agents/skills/<skill_name>/SKILL.md`. Workspace skills provide normative operational directives and verification gates:

| Skill | Purpose | Execution Trigger |
| :--- | :--- | :--- |
| **`comic-ocr-expert`** | Operational instructions, architectural doctrine, quality verification gates, status honesty rules, and master TODO roadmap for Japanese Manga & Comic OCR in Rust. | Active development, feature implementation, and CI verification gates. |

---

## 3. Context Corpus Automation (`scripts/gen-llms.py`)

### The Problem: Multi-File Context Fragmentation
In large codebases, AI agents waste time and context window tokens issuing dozens of file search and inspection tool calls to understand system doctrine, schemas, and implementation details.

### The IEPE Solution: Zero-Search Single-File Corpus (`llms-full.txt`)
IEPE provides `scripts/gen-llms.py`, an automated python script that:
1. **Recursively Scans Workspace**: Collects documentation specs, schema contracts, source code, and CI manifests.
2. **Computes SHA-256 Hashes**: Generates cryptographic payload hashes to guarantee corpus integrity and detect dirty states.
3. **Strips Noise**: Excludes vendor directories, compiled binaries, cache files, and lockfiles.
4. **Generates Dual Context Files**:
   - `llms.txt`: Compact summary manifest (~15 KB) listing all workspace modules, protocols, and file paths.
   - `llms-full.txt`: Consolidated single-file master context corpus (~300+ KB) containing full-text documents and code.

---

## 4. Operational Gains from Agent & Skill Automation

1. **Zero-Search Context Resolution**:
   - AI agents read `llms-full.txt` in a single operation, gaining 100% context of codebase doctrine, contracts, and code without needing to perform 30+ separate file search calls.
2. **Bounded Authority & Ticket-First Protection**:
   - `.agents/AGENTS.md` prevents agents from making unreviewed architectural changes, mutating contracts without authorization, or deleting failing unit tests.
3. **Repeatable Parity Verification**:
   - The `comic-ocr-parity-check` skill provides deterministic verification instructions for comparing Rust ONNX outputs against PyTorch baseline images.
