# Ingestion Loop Contract

Status: **proposed**. This document specifies the contract that every ingestion
path in this workspace must satisfy. No current implementation satisfies it in
full; the gaps are listed in [Conformance](#5-conformance-status).

The workspace has three independent implementations of the same conceptual
pipeline. They disagree about discovery, segmentation, error handling, and what
counts as a pass. This contract exists so they can converge on one shape.

| Path | Entry point | Kind |
| --- | --- | --- |
| CLI | `crates/comic-ocr-cli/src/main.rs` | batch, many pages |
| Pipeline script | `scripts/run_pipeline.py::process_images` | batch, many pages |
| REST runtime | `crates/comic-ocr-runtime/src/handlers.rs::predict_handler` | single page per request |

---

## 1. The loop

Ingestion is six stages. Every path runs the same six in the same order; a path
may only differ in how it obtains bytes (stage 1) and where it writes results
(stage 6).

```
  DISCOVER  ->  DECODE  ->  SEGMENT  ->  NORMALIZE  ->  RECOGNIZE  ->  EMIT
  PageRef[]     Decoded     Region[]     RegionCrop[]   Reading[]      PageResult
                Page
```

### Stage 1 — DISCOVER

Resolve caller input into an ordered, deduplicated list of page references.

```rust
pub enum PageOrigin {
    File(PathBuf),
    Bytes { data: Vec<u8>, declared_name: String },
}

pub struct PageRef {
    pub id: String,        // stable within a run; derived from origin
    pub origin: PageOrigin,
}
```

- **Pre**: none.
- **Post**: the returned list is deduplicated by canonicalized origin and sorted
  by a total order. Two runs over the same inputs produce the same sequence.
- **Empty is not an error.** An empty discovery result yields an empty
  `RunReport` with exit code 0, not a failure and not an implicit fallback to a
  default corpus.

> A path must not silently substitute a default input set when discovery comes
> back empty. The CLI currently falls back to `tests/data/images/`, so
> `comic-ocr` with no arguments processes the test corpus. Test fixtures must be
> requested, never assumed.

### Stage 2 — DECODE

```rust
pub struct DecodedPage {
    pub id: String,
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}
```

- **Pre**: a `PageRef`.
- **Post**: `width > 0 && height > 0`. Color is RGB8; any BGR source is
  converted at this boundary and nowhere later.
- **Failure**: `IngestError::Decode` — **per-item**, see §3.

### Stage 3 — SEGMENT

```rust
pub struct Region {
    pub id: String,
    pub bounds: Rect,       // pixel coordinates within the parent page
    pub order: u32,         // 0-based position in reading order
}
```

- **Pre**: a `DecodedPage`.
- **Post** (all mandatory):
  - `regions.len() >= 1`. A detector that finds nothing returns one region
    covering the whole page.
  - Every `bounds` lies within the page rectangle.
  - `order` is a permutation of `0..regions.len()`, assigned by
    `sort_bubble_reading_order` under the page's declared `ReadingDirection`.
    Reading order is a property of the contract, not of the detector.
- **Failure**: none. Segmentation always yields at least the fallback region.

### Stage 4 — NORMALIZE

Convert each region into the tensor-ready crops the recognizer accepts.

```rust
pub struct RegionCrop {
    pub region: Region,
    pub tiles: Vec<DynamicImage>,   // len() >= 1
}
```

- **Pre**: a `DecodedPage` and its `Region[]`.
- **Post**:
  - The crop is taken. `tiles` derive from `image.crop_imm(bounds)` — **never
    from the unsegmented page**.
  - Regions whose aspect ratio exceeds `max_aspect_ratio` are split by
    `resample_tiles`; all others yield exactly one tile.
  - Tile order preserves reading order within the region.
- **Failure**: none.

> This is the stage that does not currently exist anywhere. See §5.

### Stage 5 — RECOGNIZE

```rust
pub struct RegionReading {
    pub region_id: String,
    pub text: String,
    pub confidence: Option<f32>,        // None when not computed
    pub token_probabilities: Vec<f32>,
    pub engine: EngineType,
    pub duration_ms: f64,
}
```

- **Pre**: a `RegionCrop`.
- **Post**:
  - `text` originates from an inference call on `tiles`. No literal, no
    placeholder, no value carried over from a fixture. (Invariant **I1**.)
  - `confidence` is `Some(v)` only if `v` was computed from model output for
    *this* call. If the engine cannot produce a score, it is `None`. There is no
    default confidence.
  - `duration_ms` is measured, not declared.
  - Multi-tile regions concatenate tile readings in tile order; the region
    confidence is the geometric mean of tile confidences, or `None` if any tile
    is `None`.
- **Failure**: `IngestError::Inference` — **per-item**, see §3. An engine that
  cannot produce text returns `Err`, never `Ok` with a filler string.

### Stage 6 — EMIT

```rust
pub struct PageResult {
    pub page_id: String,
    pub width: u32,
    pub height: u32,
    pub regions: Vec<RegionReading>,
    pub text: String,                 // regions joined in reading order
    pub confidence: Option<f32>,
    pub duration_ms: f64,
}

pub struct RunReport {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: Vec<(String, IngestError)>,
    pub results: Vec<PageResult>,
}
```

- **Post**: `attempted == succeeded + failed.len()`. The report is emitted even
  when every page failed.

---

## 2. Invariants

These hold across all paths and are what the conformance tests in §4 check.

- **I1 — Nothing is reported that was not computed.** Every `text`,
  `confidence`, and `duration_ms` in a `PageResult` traces to an inference call
  made during this run. This extends the guarantee already enforced by
  `test_no_fabricated_output` from the engine to the whole loop.
- **I2 — Recognition input is always a crop.** The recognizer receives
  `RegionCrop.tiles`. It receives a full page only when segmentation legitimately
  returned the single fallback region.
- **I3 — Reading order is total and deterministic.** Independent of filesystem
  iteration order, hash iteration order, or detector output order.
- **I4 — Per-item failures are isolated.** One unreadable page or one failed
  inference does not abort the run or discard results already computed.
- **I5 — Determinism.** Same inputs, same model, same configuration produce the
  same `text` values across runs.
- **I6 — Configuration is explicit.** A missing required setting fails at
  startup with a message naming the setting. It does not resolve to an empty
  string that fails later inside a subprocess.
- **I7 — One threshold.** Quality thresholds are defined once and read by every
  gate. A path may not hold its own copy.

---

## 3. Error semantics

| Error | Stage | Class | Behavior |
| --- | --- | --- | --- |
| `Config` | startup | **fatal** | Abort before stage 1. Name the missing setting. |
| `Discovery` | 1 | **fatal** | A malformed glob is caller error. |
| `Decode` | 2 | per-item | Record in `failed`, continue. |
| `Inference` | 5 | per-item | Record in `failed`, continue. |
| `NotImplemented` | 5 | per-item | Record in `failed`, continue. Never coerced to a successful empty reading. |
| `Emit` | 6 | **fatal** | An unwritable output path is caller error. |

Exit code: `0` if `failed.is_empty()`, otherwise `1`. A run that failed every
page must not exit 0.

---

## 4. Conformance tests

A path conforms when these pass against it:

1. `discovery_is_deterministic` — shuffled input order yields identical
   `PageRef` sequences.
2. `empty_discovery_is_not_a_fallback` — no arguments processes zero pages and
   does not touch `tests/data/`.
3. `segment_always_yields_a_region` — a blank image yields exactly one
   full-page region.
4. `regions_are_within_page_bounds` — property test over random pages.
5. `reading_order_is_a_permutation` — `order` values are exactly
   `0..len`, under each `ReadingDirection`.
6. `recognizer_receives_a_crop` — a spy engine asserts its input dimensions
   equal the region bounds, not the page bounds. **This is the test that would
   have caught the current defect.**
7. `per_item_failure_is_isolated` — a corrupt page among three valid ones yields
   `succeeded == 3`, `failed.len() == 1`, exit code 1.
8. `no_default_confidence` — an engine returning no score yields
   `confidence: None`, never `0.985`.
9. `missing_model_config_fails_at_startup` — unset model name aborts before any
   image is opened.

---

## 5. Conformance status

| Requirement | CLI | Pipeline script | REST |
| --- | --- | --- | --- |
| Stage 3 SEGMENT | partial — computed, then discarded | ✗ absent | ✗ absent |
| Stage 4 NORMALIZE | ✗ absent | ✗ absent | ✗ absent |
| I2 crop-not-page | ✗ | ✗ | ✗ |
| I4 failure isolation | ✗ decode isolated, inference fatal | partial | n/a |
| I6 explicit config | ✗ empty-string default | ✗ raises `KeyError` | ✗ empty-string default |
| I7 one threshold | ✗ `0.05` | ✗ `0.20` | n/a |

### The load-bearing gap

`TextDetector::detect_regions` is invoked exactly once in the workspace, at
[`main.rs:218`](../crates/comic-ocr-cli/src/main.rs). Its result is used for
`detected_regions_count` and then dropped. The very next line recognizes the
**whole page**:

```rust
let regions = TextDetector::detect_regions(&img);   // computed
let result = engine.predict(&img)?;                 // ...and ignored
```

`resample_tiles` and `sort_bubble_reading_order` are exported from
`comic-ocr-core` and called by nothing outside their own unit tests. Stages 3
and 4 are written but unwired.

This has a visible consequence. `run_pipeline.py` maintains
`KNOWN_FAILING_SPREADS` — `12.jpg`, `13.jpg`, `14.jpg` at CER 0.91–1.00 —
annotated as needing "multi-koma region segmentation" and "bubble segmentation".
That is precisely stage 3 plus stage 4. `manga-ocr-base` is trained on
single-bubble crops; a full page cannot produce a correct reading from it. These
are not model limitations and not known-bad fixtures. They are the missing crop,
and the allowlist currently converts that defect into an expected result — so
the gate will stay green through exactly the fix that should turn it green
honestly.

### Secondary gaps

- **No working default model.** `OrtEngine::new(env::var("COMIC_OCR_MODEL").unwrap_or_default())`
  yields `""` when unset, which reaches the subprocess as an empty
  `COMIC_OCR_MODEL_NAME` and fails with
  `OSError: Incorrect path_or_model_id: ''` after the image has already been
  decoded and written to a temp file. Violates I6. `run_pipeline.py` uses
  `os.environ[...]` and raises `KeyError` instead — same defect, different
  message.
- **Two thresholds.** The Rust gate passes items at `cer <= 0.05`; the Python
  gate at `cer <= 0.20`. The Rust gate also prints its threshold as `0.05%` when
  it means 5%. Violates I7.
- **The Rust gate makes claims from stored data.** `--gate` reads
  `benchmark_results.json` and prints `TEST SUITES PASSED` without running any
  inference. `run_pipeline.py` explicitly refuses this, skipping with
  "without making claims based on stored static ledger data" when no backend is
  present. The Rust gate should adopt the same refusal — reporting a pass from a
  recorded number is the same shape as the fabricated outputs this repo has
  been removing.
- **Inference failure aborts the CLI run.** `engine.predict(&img)?` propagates,
  discarding results already computed for earlier pages, while a failed
  `image::open` merely logs and continues. Violates I4.
- **Model reload per image.** `OrtEngine::predict` spawns `python3` and loads
  the model on every call, so a 20-page run pays the load 20 times.
  `run_pipeline.py` loads once and loops — the Python path is currently the
  faster of the two. A persistent worker (already on the Phase 9 roadmap) is
  what makes stage 5 viable per-region, since correct segmentation multiplies
  inference calls by roughly the region count.

---

## 6. Suggested sequencing

1. Wire stages 3–4 in the CLI: crop to `regions`, recognize per region, join by
   reading order. Add conformance test 6 first — it fails today.
2. Add the persistent inference worker. Per-region recognition makes per-call
   model loading untenable.
3. Unify the gate: one threshold constant, one code path, no claims from stored
   data.
4. Re-measure `12/13/14` and retire `KNOWN_FAILING_SPREADS` on evidence.
5. Extract the loop into `comic-ocr-core::ingest` so the CLI, script, and REST
   runtime share one implementation rather than three.
