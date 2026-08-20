# Integrating comic-ocr with the Infinite Verse ingest pipeline

Draft, 2026-08-20. Written against `manga-service` at `45750c8` and this repo at
`1ff0726`.

## What already exists on the other side

`manga-service` runs a durable detection queue (`vision_detection_job`) that
dispatches on a `job_kind` discriminator. As of #519 it carries three engines,
and adding a fourth is a const, a dispatch arm, and an enqueue function:

| engine | kind | what it writes |
| --- | --- | --- |
| `image-stats` | inline at commit | page metrics |
| `vision-worker` | `region-detection` | `regions[]` — panels and text, via Gemini |
| `ocr-detector` | `ocr` | `textLayers[]` — transcriptions, via Gemini |
| `panel-detector` | `panel-detection` | `regions[]` — panels, deterministic |

Every engine writes a `page-semantics` envelope to CAS and registers it against
the reading item. Multiple engines attach to the same page and **none supersedes
another** — that multiplicity is deliberate (migration `0021`), and it is what
makes a second opinion visible rather than destructive.

Two facts about that seam matter for this repo:

**Region ids must be namespaced.** The broker merges envelope sections by region
id and the last document wins, so two engines both emitting `text-001` do not
disagree — one silently replaces the other. `vision-worker` emits unprefixed ids
and must keep doing so (its ids are referenced by `textLayers[].regionId` across
the corpus). Everything else carries a prefix; `panel-detector` uses `pd-`. This
integration uses **`co-`**.

**Assertion state is a ledger, not a flag.** The schema's vocabulary is
`candidate | accepted | verified | rejected`. Machine output is always
`candidate`. Nothing promotes itself.

## Where comic-ocr runs

As its own Railway service inside the `anime-universe` project, so
`comic-ocr-runtime.railway.internal` resolves — Railway private networking is
per-project, so it cannot live in a different project and be called internally.
This mirrors `blob-service` and `auth-service`.

`comic-ocr-runtime` already exposes what is needed:

```
POST /v1/ocr/predict?extract_furigana=bool   multipart image  → {text, confidence, duration_ms}
GET  /v1/runtime/health                                       → {status, metrics:{total_failed_ocr,…}}
```

## Two things the consumer must not trust yet

These are consequences of `OrtEngine::predict` as it stands, and the integration
is written to survive them rather than launder them.

**`confidence` is a constant.** `predict()` returns `confidence: 0.985`
unconditionally — on success, and on total inference failure. Until it carries a
real per-token probability, the client below **drops the field entirely** rather
than writing a fabricated 0.985 into every transcription in the corpus. An
envelope with no confidence is honest; one with a fake one is not, and
downstream ranking would believe it.

**Failure returns `Ok`.** A missing `python3`, a failed model download, or a
script error yields `Ok(OcrResult { text: "…" })`. The client therefore treats
the sentinel as a failure defensively — belt and braces until the `Err` path
lands, because the alternative is 830 pages transcribed as `…` at 98.5%
confidence, entering the graph as candidates and then feeding cross-engine
agreement.

Both are cheap to fix here, and when they are, the two guards below become
redundant rather than wrong. Leave them.

## The trait boundary

The point of this shape is that **failure has to be a value the caller can see**.

```rust
/// One page's text, transcribed. Errors are errors.
pub trait TranscriptionEngine {
    fn transcribe(&self, image: &DynamicImage, opts: &TranscribeOptions)
        -> Result<Transcription, TranscribeError>;
}

pub struct Transcription {
    pub text: String,
    /// The model's own probability, when it produced one. `None` is the correct
    /// answer for an engine that does not measure confidence — never a default.
    pub confidence: Option<f32>,
    pub duration_ms: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    #[error("inference backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("inference failed: {0}")]
    InferenceFailed(String),
    #[error("image could not be decoded: {0}")]
    Decode(String),
    #[error("the backend returned no usable text")]
    EmptyResult,
}
```

`OrtEngine` implements this by returning `Err(BackendUnavailable)` where it
currently returns `"…"`, and `confidence: None` until real token probabilities
are threaded through. Nothing else in the workspace changes shape.

## Mode A first: transcribe what is already found

`comic-ocr-core::TextDetector::detect_regions` means this repo *can* find its own
text regions, so there are two possible integrations. Do the smaller one first.

**Mode A — transcriber.** Consume the text regions `vision-worker` already found
and transcribe each one. This slots exactly where `ocr-detector` sits, chains off
the same region envelope, and — the reason it goes first — makes comic-ocr
**directly comparable to the incumbent on identical inputs**. That is what
"replace or join" needs to be decidable at all.

**Mode B — detector and transcriber.** Find text regions too, as a peer of
`vision-worker` rather than a consumer. Worth doing, but only once Mode A has
shown the transcription is good; otherwise a disagreement is unattributable
between detection and reading.

### The manga-service side of Mode A

```rust
// vision_worker.rs — identity, beside PANEL_ENGINE
pub const TRANSCRIBER_ENGINE: &str = "comic-ocr";
pub const TRANSCRIBER_VERSION: &str = "0.1.0";
pub const TRANSCRIBER_RELEASED_AT: &str = "2026-08-20T00:00:00Z";
/// Namespaced for the merge. See DetectorIdentity::id_prefix.
pub const TRANSCRIBER_ID_PREFIX: &str = "co-";
pub const JOB_KIND_TRANSCRIPTION: &str = "transcription";

/// Distinct namespace — the region key is a bare "{VERSION}:{digest}", so an
/// unprefixed fourth kind could collide and be swallowed by ON CONFLICT.
pub fn transcription_idempotency_key(resource_keys: &[String]) -> String {
    format!("{}:{}:{}", JOB_KIND_TRANSCRIPTION, TRANSCRIBER_VERSION,
            unit_digest(resource_keys))
}
```

Dispatch is one arm in `run_worker`, next to the panel arm:

```rust
} else if job.job_kind == JOB_KIND_TRANSCRIPTION {
    process_transcription_job(&context, &job).await.map(|o| o.diagnostics())
}
```

The job body reuses machinery that already exists. Verified present in
`manga-service` at `45750c8`: `fetch_page_bytes`, `build_detection_canvas`,
`upload_envelope_blob`, `register_envelope`, `unit_digest`, `DetectorIdentity`,
`Submission::Full`, and `extract_text_regions` (`ocr_worker.rs:104` — the OCR
worker already turns a region envelope into transcription subjects, so Mode A
inherits it whole).

**Four symbols in the sketch below do not exist and have to be written**, named
here so nobody greps for them and concludes the plan is further along than it is:
`VisionError::TranscriptionFailed`, `crop_to_region`, `region_envelope_for`, and
`DetectionOutcome::empty`. `WorkerContext.transcriber` is new as well — and note
it should be `Option<…>`, for the same reason `config` became optional in #519:
a worker that cannot be constructed without a backend leaves every other job kind
dark when that backend is absent.

```rust
async fn process_transcription_job(context: &WorkerContext, job: &ClaimedJob)
    -> Result<DetectionOutcome, VisionError>
{
    // The text regions vision-worker already found, from the envelope this job
    // was chained off — same input the incumbent gets.
    let subjects = extract_text_regions(&region_envelope_for(context, job).await?);
    if subjects.is_empty() {
        return Ok(DetectionOutcome::empty());   // absent stays absent
    }

    let pages = fetch_page_bytes(context, &job.image_urls).await?;
    let (canvas, mime, w, h) = build_detection_canvas(
        &pages, job.unit_kind == "spread", &job.reading_direction, Submission::Full)?;

    let mut lines = Vec::new();
    for region in &subjects {
        let crop = crop_to_region(&canvas, region, w, h)
            .map_err(|e| VisionError::Decode(e.to_string()))?;
        match context.transcriber.transcribe(&crop, &opts).await {
            Ok(t) => lines.push((region.id.clone(), t)),
            // One unreadable balloon is not a failed page. It is recorded as an
            // absence on that region and the rest of the page still lands.
            Err(e) => tracing::warn!(region = %region.id, error = %e,
                        "[comic-ocr] region unreadable"),
        }
    }
    if lines.is_empty() {
        return Err(VisionError::TranscriptionFailed(
            "no region on this page could be read".into()));
    }
    // …build envelope, upload, register under TRANSCRIBER_ENGINE / "co-" …
}
```

### The envelope it writes

`textLayers[]`, keyed on the region ids `vision-worker` produced, so the two
transcriptions of the same balloon are directly comparable:

```jsonc
{
  "version": "1.0",
  "scope": { "publicationId": "…", "readingItemId": "…", "locator": "…" },
  "access": { "regions": "open", "textLayers": "entitled" },
  "regions": [ /* the synthetic page region, as the other writers emit */ ],
  "textLayers": [{
    "id": "co-ocr-ja",
    "kind": "ocr",
    "language": "ja",
    "regions": [{
      "id": "co-text-001",
      "regionId": "text-001",        // vision-worker's id — the join key
      "text": "…",
      "direction": "ttb",
      "ruby": [ /* furigana, which is the part Gemini does worst */ ],
      "state": "candidate"
      // no `confidence` until it is measured
    }]
  }],
  "provenance": {
    "records": { "comic-ocr": {
      "source": "ocr",
      "engine": "comic-ocr",
      "model": "…",                  // omit entirely if no model ran
      "createdAt": "2026-08-20T00:00:00Z"   // release constant, not wall clock
    }},
    "fields": { "/textLayers": "comic-ocr" }
  }
}
```

`createdAt` being a release constant is what keeps the envelope digest stable
across re-runs over identical bytes — every existing writer follows this, and a
wall clock there silently makes every re-run a new document.

## What decides replace-or-join

Both engines transcribe the same regions on the same pages. The comparison is
then direct, and it is worth reporting per-region rather than as an average:

- **agreement rate** on identical region ids
- **furigana**: `ocr-detector` gets ruby from prompt engineering;
  `comic-ocr-core` has an FSM. This is where a local engine should win outright.
- **vertical text**: the reader already collapses `vertical` to `ltr` (a known
  gap in `live.ts`); a transcriber that reports `ttb` correctly is worth more
  than one that guesses.
- **cost and failure**: `ocr-detector` inherits Gemini's rate limit — 181 region
  jobs burned all five attempts inside one window on 2026-08-18. A local engine's
  failure modes are different in kind, not just in rate.

## Deployment notes

- **The model reloads per call.** `from_pretrained` sits inside the generated
  Python script, so every request pays a full load. Until that is hoisted, the
  client needs a long timeout and concurrency of 1, and the queue's existing
  lease (`LEASE_SECONDS = 180`) is the ceiling on how slow a single page may be.
- **Container size.** A Python + torch + transformers image is ~2GB. A genuine
  `ort` path with a quantised model is the difference between that and ~100MB,
  and it is the difference between this being deployable per-page and not.
- **No API key needed**, which means this engine runs on an environment where
  Gemini is unconfigured. `run_worker` now spawns on a database rather than on a
  key, so a transcription job is claimable there — the same property that lets
  `panel-detector` work today.

## Order of work

1. `Err` on failure, `Option<f32>` confidence — in this repo. Everything else
   depends on it, and the two defensive guards above exist only because of it.
2. Deploy `comic-ocr-runtime` into `anime-universe` staging; confirm
   `/v1/runtime/health` from `manga-service` over private networking.
3. Mode A: job kind, dispatch arm, enqueue, envelope. Chained off the same
   region envelope `ocr-detector` uses.
4. Score the two transcribers against each other on the same regions, and report
   the disagreements rather than the mean.
5. Mode B, if Mode A earns it.
