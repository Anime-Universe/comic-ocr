# iPub Format Architecture & Infinite Verse Integration Specification

This document provides a comprehensive, code-level specification of the **iPub Publication Format** and its exact integration boundary between **Comic OCR Rust** (`comic-ocr-rust`) and **Infinite Verse** (`Infinite-Platform/services/manga-service` and `universe-broker`).

---

## 1. Executive Architecture & Governing Principles

**iPub** (`.ipub`) is the canonical, content-addressed publication format across the Infinite Verse ecosystem.

### Governing Split
$$\mathbf{\text{iPub}} = \text{publication semantics}, \qquad \mathbf{\text{CAS}} = \text{immutable resource bytes}$$

- **Content-Addressed Storage (CAS)**: Page images (PNG, WebP, AVIF), font assets, and binary payloads do **not** live inside the iPub manifest object. They remain independently addressable CAS resources named by their content digests (BLAKE3-256 / SHA-256).
- **Manifest Responsibility**: The iPub manifest declares publication identity, work relationships, edition geometry, reading order, and references to CAS digests.

---

## 2. iPub Specification & Federation Boundaries

### Normative Authority
In accordance with `docs/ipub-docs/00-README.md` in Infinite Verse:
1. **Specification**: `03-ipub-specification.md` (The logical model and normative rules).
2. **Machine Schema**: `schema/publication.schema.json` (The JSON Schema 2020-12 contract).

### Strict Federation Boundary (`21-ipub-boundary.md`)
An iPub manifest **MUST NOT** contain:
- User identities, accounts, or session tokens
- Entitlements, payment history, or subscription state
- Service topology (internal hostnames, staging buckets, secrets, or temporary signed URLs)
- Local filesystem paths

User entitlements, purchases, and reading progress exist in separate platform layers and reference iPub identity rather than extend it.

### Conformance Modules & Presentation Profiles
Conformance is structured along two orthogonal axes:
- **Capability Modules**: Core (Required), Bibliographic, Navigation, Physical, Semantic (`#519`/`#520`), Accessibility.
- **Presentation Profiles**: Fixed-Page (Manga, Western Comics, PDF), Reflowable (EPUB), Spatial (2.5D/3D composite panels - deferred).

---

## 3. Infinite Verse Database Substrate (PostgreSQL / AlloyDB)

Defined in [`Infinite-Platform/services/schema.sql`](file:///Users/zachshallbetter/Projects/Infinite-Verse/Infinite-Platform/services/schema.sql) and migrations `0002`, `0016`, `0020`, `0021`.

### CAS Asset Manifest Table
```sql
CREATE TABLE asset_manifests (
    asset_id VARCHAR(128) PRIMARY KEY CHECK (asset_id ~ '^b[a-z2-7]+$'),
    manifest_version VARCHAR(16) NOT NULL DEFAULT '1.0',
    hash_algorithm VARCHAR(32) NOT NULL DEFAULT 'blake3-256',
    media_type VARCHAR(255) NOT NULL,
    byte_length BIGINT NOT NULL CHECK (byte_length >= 0),
    chunk_size BIGINT NOT NULL CHECK (chunk_size > 0),
    compression VARCHAR(32) NOT NULL DEFAULT 'zstd-independent-frames',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
```

### Multi-Engine Attachment (Migration `0021`)
Multiple detection/transcription engines (`vision-worker`, `panel-detector`, `comic-ocr`, `annotation-surface`) attach independent candidate envelopes to the same reading item without destructive overwrites:

```sql
-- reading_item_resource primary key allows multiple engine attachments
PRIMARY KEY (revision_id, item_id, resource_id, resource_role)
```

- **`publication_resource`**: Scoped to the whole publication revision.
- **`reading_item_resource`**: Scoped to a page or spread item with `resource_role = 'page-semantics'`.
- **Side-Index Mutability (Migration `0020`)**: Permits `INSERT` and `DELETE` of `page-semantics` rows on published revisions without modifying the frozen manifest.

---

## 4. The `IPubSemanticResource` Envelope (`page-semantics`)

Defined in `docs/ipub-docs/schema/semantic-resource.schema.json`:

```jsonc
{
  "$schema": "https://schemas.infiniteverse.ai/ipub/1.0/semantic-resource.schema.json",
  "version": "1.0",
  "scope": {
    "publicationId": "pub_01...",
    "readingItemId": "item_001",
    "locator": "page-1"
  },
  "regions": [
    {
      "id": "co-001",
      "kind": "text",
      "bounds": { "x": 100, "y": 150, "width": 80, "height": 300 },
      "normalizedBounds": [0.10, 0.15, 0.18, 0.45],
      "state": "candidate"
    }
  ],
  "textLayers": [
    {
      "id": "tl-co-001",
      "language": "ja",
      "kind": "transcription",
      "regions": [
        {
          "id": "co-text-001",
          "regionId": "text-001",       // Join key referencing vision-worker's region ID
          "text": "冒険の始まり...",
          "direction": "ttb",
          "ruby": [
            { "base": "冒険", "text": "ぼうけん" }
          ],
          "state": "candidate"
        }
      ]
    }
  ],
  "provenance": {
    "records": {
      "comic-ocr": {
        "source": "ocr",
        "engine": "comic-ocr",
        "model": "manga-ocr",
        "engineVersion": "0.1.0",
        "createdAt": "2026-08-20T00:00:00Z"   // Release constant for stable JCS byte hashing
      }
    },
    "fields": {
      "/textLayers": "comic-ocr"
    }
  }
}
```

### Critical Envelope Invariants
1. **Assertion State Ledger**: `candidate | accepted | verified | rejected`. Machine predictions from `comic-ocr` default to `candidate`.
2. **Region ID Namespacing**:
   - `vision-worker` (unprefixed: `text-001`, `panel-001`)
   - `panel-detector` (prefix: `pd-`)
   - `comic-ocr` (prefix: **`co-`**, e.g., `co-001`, `co-text-001`)
3. **Deterministic JCS Hashing**: `provenance.createdAt` uses a fixed version release constant (`TRANSCRIBER_RELEASED_AT = "2026-08-20T00:00:00Z"`). The envelope is canonicalized via JCS (RFC 8785) and hashed (SHA-256) to ensure byte-deterministic CAS indexing across re-runs.

---

## 5. Integration Architecture: Mode A vs. Mode B

```
Mode A (Transcriber) [Primary Integration]:
  [vision-worker (Gemini)]  --->  Emits Region Bounding Geometry
                                        |
                                        v
                               [comic-ocr-rust]  --->  Transcribes cropped regions into textLayers[].regions[]

Mode B (Detector + Transcriber) [Peer Engine]:
  [comic-ocr-rust]  --->  Performs both TextDetector geometry AND OCR transcription natively
```

- **Mode A (Transcriber)**: Consumes region envelopes produced by `vision-worker` (`manga-service/src/ocr_worker.rs::extract_text_regions`), crops image targets to `normalizedBounds`, calls `comic-ocr-runtime`, and emits `textLayers[].regions[]` referencing `regionId`. This allows direct, side-by-side benchmark comparison between `comic-ocr` and `ocr-detector` (Gemini OCR) on identical input regions.
- **Mode B (Detector + Transcriber)**: Runs `TextDetector` natively to produce both region bounding geometry and text transcriptions as a peer detector.

---

## 6. Human Correction Engine & Reader Security (`universe-broker`)

Source file: `Infinite-Platform/brokers/universe-broker/src/semantics.rs`

### Corrections Endpoint (`POST .../semantics/corrections`)
- **Engine Identifier**: `ENGINE = "annotation-surface"`, `ENGINE_VERSION = "1.0.0"`.
- **Correction Operations**:
  - `region.add`: Human-drawn region.
  - `region.adjust`: Adjusted bounding geometry.
  - `region.delete`: Tombstone transition setting state to `rejected`.
  - `text.correct`: Transcription, translation, or speaker attribution correction.
  - `state.set`: State transition (`candidate` $\rightarrow$ `accepted | verified | rejected`).
- **Access Class Security**:
  - `open` sections: `regions`, `observations`, `metrics`, `relations`.
  - `entitled` sections: `textLayers`, `entities`, `appearances`.
  - Mechanical guard refuses any envelope attempting to smuggle raw text inside `open` sections.

---

## 7. Verification & Conformance Stack

To verify `comic-ocr-rust` against this specification:

```bash
# 1. Run Workspace Unit & Integration Tests (87 passed, 2 ignored)
cargo test --workspace

# 2. Strict Workspace Clippy Audit
cargo clippy --workspace --all-targets -- -D warnings

# 3. Regenerate Context Corpora
python3 scripts/gen-llms.py
```
