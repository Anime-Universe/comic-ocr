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
│   ├── llms-full.txt           # ~300+ KB Single-File Consolidated Workspace Corpus
│   └── skills/                 # Custom Agent Skills
│       ├── initialize-iepe-project/
│       ├── operate-iepe-project/
│       ├── qualify-iepe-outcome/
│       ├── comic-ocr-parity-check/
│       └── stress-test-iepe-candidate/
└── scripts/
    └── gen-llms.py             # Single-File Context Corpus Generator Script
```

---

## 2. The Skill Taxonomy & Lifecycle

Skills represent specialized instruction manuals stored as `.agents/skills/<skill_name>/SKILL.md`. Each skill addresses a specific phase of the engineering workflow:

| Skill | Purpose | Execution Trigger |
| :--- | :--- | :--- |
| **`initialize-iepe-project`** | Bootstraps workspace governance contracts and `.agents/` directory structure. | New project adoption or workspace initialization. |
| **`reconcile-iepe-project`** | Fixes state drift between git commits, open issues, and recorded artifacts. | First cycle of a turn or detected state inconsistency. |
| **`operate-iepe-project`** | Guides step-by-step execution of issue contracts under bounded authority. | Primary coordinator loop during active development. |
| **`qualify-iepe-outcome`** | Verifies empirical test evidence before merging or promoting code. | Pull request review and release candidate gates. |
| **`stress-test-iepe-candidate`**| Executes controlled adversarial trials (corrupted images, extreme aspect ratios). | Validation phase before feature promotion. |
| **`comic-ocr-parity-check`** | Compares Rust ONNX outputs against PyTorch baseline to verify 0% CER divergence. | Parity verification gate in CI/local runs. |
| **`maintain-iepe-package`** | Re-compiles `llms.txt` and `llms-full.txt` corpora after code/doc updates. | Post-refactoring or document modification. |

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
