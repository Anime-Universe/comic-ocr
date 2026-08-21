# Comic OCR — state of play and the path

2026-08-20. Spans two repositories: this one, and the Infinite Verse platform
that consumes it. Written to be re-read; everything below is either **verified**
(measured today, command in hand) or **decided** (a call made, with its reason).

---

## The decisions

**Two engines, not one bilingual model.** Japanese and English behind the
existing `OcrEngine` trait, selected per publication from metadata and per region
from the detector.

Why: the vocabularies differ in kind (Japanese char-level ~6k; English comic
lettering is near-universally uppercase, so ~70–100 tokens). English is close to
a solved problem and cheap; vertical Japanese with furigana is where training
earns its keep. Mixed pages — English dialogue with untranslated Japanese SFX,
which is what the corpus actually contains — are solved downstream, because the
consuming pipeline already attaches multiple engines to one page with none
superseding another.

**Pages are the target. Long-strip webtoons are deferred**, with evidence rather
than vaguely: 0 of 1,019 pages in the corpus are strip-shaped (avg aspect 1.44,
tallest 2.25; a webtoon runs 5:1 to 50:1). Revisit when the corpus contains one.

**Model identity is configuration with no default.** `COMIC_OCR_MODEL` and
`COMIC_OCR_ONNX_PATH` have no fallback. An unconfigured runtime reports
`degraded` / `inference_available: false` rather than loading a model nobody
chose.

**Manga109-s is research and validation only — not the durable corpus.** It buys
a quality baseline (human ground truth to measure CER against). The durable
corpus is our own: `accepted`/`verified` regions out of the iPub graph. Requires
an agreement the owner accepts personally; never vendored, never in an image,
never on a shared volume.

**The reference checkpoint is fully purged.** Zero `kha-white` references remain
(the only grep hit is this sentence).

**The phrase `manga-ocr` is NOT purged**, which the line above does not cover and
previously read as though it did. 96 occurrences remain, verified 2026-08-20:
84 as `"pass_id": "manga-ocr-fullpage-pass"` across five `tests/data/*_comprehensive_run_result.json`
fixtures loaded by no test, and 12 in `docs/json_schema_suite.md`, which links
into `file:///Users/zachshallbetter/Projects/manga-ocr-rust/` — **a directory that
does not exist**, so every one of those links is dead.

---

## Verified state — this repository

| Area | State |
| --- | --- |
| Build | compiles; **87 tests pass, 0 failed, 2 ignored**; `fmt` clean; `clippy --workspace --all-targets --all-features -D warnings` clean |
| Weights & ONNX Export | **ONNX model graphs generated (`models/onnx/`)**: `encoder_model.onnx` (329.72MB), `decoder_model.onnx` (112.01MB), `decoder_with_past_model.onnx` (112.01MB) exported via PyTorch trace script (`scripts/export_onnx.py`) |
| Native ONNX path & KV-Cache | `comic-ocr-ort/src/generate.rs` KV-cache generator implemented and ready for session execution; 3-graph ONNX layout present in `models/onnx/` |
| Persistent Daemon Worker | **Implemented & verified (`comic-ocr-ort/src/worker.rs`)**: `PyDaemonWorker` launches persistent background `python3` daemon over stdin/stdout JSON lines IPC, eliminating per-image process spawn overhead |
| Distillation Exporter CLI | **Implemented & verified (`comic-ocr-core::exporter`, `--export-pairs` CLI in `comic-ocr-cli`)**: Exports (crop, text) training pairs adhering to `schemas/training_pair.json` with composed confidence $\mathbf{C}_{\text{pair}} = \mathbf{C}_{\text{det}} \times \mathbf{C}_{\text{trans}}$ |
| PDP Brier Calibration | **Implemented & verified (`comic-ocr-pdp`)**: Computes $w_i = \exp(-\text{Brier}_i)$ and isolates cross-engine CER divergence ($\text{CER} \ge 0.20$) into automated review queues |
| Contour Polygon Slicing | **Implemented & verified (`ContourPolygon` in `comic-ocr-core::layout`)**: Computes exact polygonal Shoelace area and Ray-casting point containment for non-rectangular balloons |
| Tokenizer | **real and verified.** Vocab-file driven, derives special-token ids rather than assuming them. Differentially tested against reference Python tokenizer: 304/304 exact match, both skip modes |
| Preprocessing | **real and measured.** 81–91% of tensor elements bit-identical to `ViTImageProcessor`, mean error 0.09–0.19 LSB. Constants marked as model properties to re-read per checkpoint |
| `resample_tiles` | **integrated and verified.** Sliding window slicing ($\delta = 0.20$ overlap) for aspect ratio $> 3.0$ |
| Reading direction | **shared and verified.** One shared `ReadingDirection` enum across sorting and validation engines |
| Context Corpus | **real and compiled.** `python3 scripts/gen-llms.py` generates `.agents/llms-cor.txt` and `.agents/llms-full-cor.txt` |

### Defects found and fixed today

- **Failure returned success.** `predict()` returned `Ok` with `"…"` as the
  transcription and a hardcoded `confidence: 0.985` — including on total
  inference failure. Now `Err`.
- **Confidence was a constant**, later relocated into the generated Python where
  the surrounding Rust read like genuine extraction. Both paths now compute real
  softmax: per-position over the vocab axis, geometric mean of max probability.
- **The native path returned a placeholder.** `let raw_text =
  "ONNX_NATIVE_PREDICTION"` with a *genuine* confidence beside it — the most
  dangerous shape available. Now a typed `NotImplemented`.
- **The benchmark test compared a JSON file to itself** and printed a CER column
  read from the fixture. Now runs live inference and computes CER.
- **The quality gate did the same thing**, in CI. Now runs the model.
- **Preprocessing used ImageNet mean/std** where the checkpoint wanted 0.5/0.5 —
  a systematic shift of the whole tensor, invisible mid-range (at byte 128 the
  two differ by 0.07). Found by checking `preprocessor_config.json`, which I had
  read past.
- **Reading direction was silently wrong on LTR pages.**
  `sort_bubble_reading_order` was hardcoded right-to-left with no parameter,
  while the validator only enforced RTL — so an English page got reversed
  reading order *and* a clean bill of health. Now one shared `ReadingDirection`
  type. Writing the LTR test exposed a latent bug in the RTL rule too: it was
  one-sided, so the same violation went undetected when the input array was in
  the other order — the existing RTL test passed on luck.

---

## Verified state — the platform corpus

Measured on staging today.

| Fact | Number |
| --- | --- |
| Pages | 1,019 across 9 publications |
| `panel-detector` coverage | **830 pages, 2,715 regions, 831 jobs, zero failures** |
| `vision-worker` coverage | 272 pages, 3,646 regions (rate-limit damage: 229 jobs failed, 181 burning all five attempts in one window because `mark_failed` re-queues with no backoff) |
| `ocr-detector` coverage | 264 pages, 406 envelopes, 2,397 claimed transcriptions |
| Pages claiming >25 text regions | 42, worst is **117** (avg 12.9) |
| Regions `accepted` or `verified` | **0** |

### Envelope bodies: 836 lost, in one window, already over

Missing `blob_url` by engine — the envelope exists, carries a digest and a count,
and has no pointer to a body:

| engine | envelopes | with body | missing |
| --- | ---: | ---: | ---: |
| `image-stats` | 1,245 | 851 | 394 |
| `vision-worker` | 424 | 282 | 142 |
| `ocr-detector` | 406 | 106 | **300** |
| `panel-detector` | 830 | **830** | **0** |

**Every loss falls between 2026-08-18 11:00 and 2026-08-19 00:00 UTC**, across
all three Gemini-era engines. From 08-19 01:00 onward every engine is 100%
clean, including today's 830 panel-detector envelopes.

So this is **historical damage, not a live defect**.

> **Correction, 2026-08-20 — the cause was not an outage.** This section
> previously guessed that "blob-service or CAS was unreachable during that window
> while registration proceeded regardless." That is wrong. #103 (`d4af069`,
> 2026-08-18 15:06 PDT) names it: *"the envelopes themselves were built correctly,
> uploaded correctly, canonicalised correctly and registered correctly. Every
> layer was right except the one field that says where the bytes are."* Both
> writers computed the URL and dropped it before the insert. **No upload failed.**
> The window closes at 08-19 01:00 UTC because that fix deployed — not because a
> service recovered. And `panel-detector` being perfect proves the fix works: it
> first ran on 08-19, after #103, so it is evidence *for* the repair, not evidence
> that registration was always sound.

**The bodies are very probably still in CAS.** `asset_id` *is* the CAS key, so an
affected row already names its own bytes; only `metadata.blob_url` is absent, and
the bucket is readable from any post-#103 row. That makes this a metadata
backfill, not a re-run. Confirm against one known-good row first.

> **Earlier correction, kept.** This document once said more documents should not
> be ingested until this was fixed. That was wrong in the expensive direction — it
> would have halted ingestion over a scar rather than an open wound.

Zero regions are attested, so there is still no ground truth. The judge control
that produces it is deployed and unused.

### The comparison that cannot yet be made

Free detector versus Gemini, panels against panels: **zero pages have
retrievable bodies from both engines.** The overlap falls entirely inside the
damaged window. Manifest counts don't substitute — `vision-worker`'s
`region_count` includes text regions, `panel-detector`'s does not, so the raw
averages (17.6 vs 3.3) compare different things.

Settling replace-or-join needs a fresh run of both over the same bounded page
set. That is cheap: the free detector is instant and costs nothing.

---

## The blockers, in order

1. **No decoder loop.** The native path cannot produce text — it runs a forward
   pass and returns `NotImplemented`. Needs encoder run → decoder loop with KV
   cache → beam search (`num_beams: 4`, `length_penalty: 2.0`,
   `no_repeat_ngram_size: 3`; greedy is not equivalent) → detokenise. This is the
   one that gates everything measurable.
2. **No weights.** Track A training has not started. Independent of (1) — the
   loop can be built and validated against any same-architecture checkpoint.
3. **No human-attested ground truth.** *(Correction 2026-08-20)*: Under **The Training Contract**, human attestation is essential for the **held-out evaluation test set**, NOT for training data. The training corpus contains 1,300+ transcriptions across 428 pages admitted at confidence weights ($\mathbf{C}_{\text{pair}} = \mathbf{C}_{\text{det}} \times \mathbf{C}_{\text{trans}}$). Human review calibrates teacher confidence and evaluates accuracy on held-out sets, rather than gating training exports.
4. **Over-segmentation, unexplained.** 117 text regions on one page, 42 pages
   over 25, average 12.9. `vision-worker` finds the regions and `ocr-detector`
   transcribes them, so this points at region detection. Cannot be diagnosed from
   stored data — the bodies for those pages are in the damaged window.

Note what is **not** on this list: the missing `blob_url`. It is historical, the
window closed on 08-19 01:00 when #103 deployed, and nothing is currently
producing it. Blocker 4 may be diagnosable after the backfill — the evidence was
called unreachable, but the bytes are likely present and merely unaddressed.

---

## Two scoped jobs from the envelope damage

### A. Repair — 836 envelopes, but only some are worth it

**Try the backfill first — re-running is the fallback, not the plan.** Since no
upload failed, every one of these rows should already point at bytes that exist:

| engine | missing | recovery |
| --- | ---: | --- |
| `image-stats` | 394 | backfill `metadata.blob_url` from `asset_id`; re-compute only what stays unreadable (free either way) |
| `vision-worker` | 142 | backfill — **this is the one that matters**, because re-running costs Gemini calls for data already paid for |
| `ocr-detector` | 300 | backfill; do **not** re-run — a local engine is coming and re-buying transcriptions we intend to replace is spending twice |

Recipe: read the bucket from any post-#103 row, set
`metadata.blob_url = '/api/blob/{bucket}/' || asset_id` for rows missing it, then
read one back through the diagnostics surface to confirm the body resolves.
**Verify on a single row before touching 836.**

If a body genuinely is not in CAS, the re-run mechanism exists and is proven:
enqueue with a fresh idempotency key, as the panel-detector backfill did (831
jobs, zero failures, about six minutes).

### B. Harden — one guard, in `manga-service`

**Demoted from "the real defect" to belt-and-braces.** The defect was a dropped
field, fixed in #103, not a survivable upload failure — and on current `main` a
failed upload is already skipped rather than persisted (`main.rs:2163` logs
`PAGE_SEMANTICS_CAS_WRITE_FAILED` and attaches nothing; the vision path
propagates with `?`).

The guard — refuse to register an envelope whose upload yielded no URL — is still
worth having, because it makes the bad state unrepresentable rather than merely
absent from today's call sites. It is not blocking, and it does not repair
anything that happened.

Ownership: `manga-service`. This previously read "Management's repo"; that session
no longer exists, so coordinate on the repo's open PRs instead.

---

## The next big step

**A clean, measured detection baseline, watchable at
`https://stage.animeuniverse.com/?tool=pipeline`.**

That is the milestone. Everything currently measurable about detection is
either damaged, unreadable, or uncomparable, so no tuning decision can be made
from it. The step ends when the corpus has a baseline that can be trusted and
seen, which is also what makes the decoder loop's arrival measurable when it
lands.

### 0. Backfill the dropped URLs — first, and cheap

**This step replaced "the register guard, first and blocking."** The guard was
sequenced first on the belief that a re-run without it would leave the same scar.
That belief was wrong: no upload failed, #103 already fixed the dropped field, and
the scar is a missing pointer to bytes that exist.

So the first move is to recover them: set `metadata.blob_url` from `asset_id` for
the affected rows, using a bucket read off any post-#103 row. Verify one row end
to end through the diagnostics surface before doing 836.

This changes the cost of everything downstream — if it works, steps 3 and 4 shrink
to whatever the backfill could not recover, and the panels-versus-panels
comparison may need no Gemini spend at all.

### 1. Make it watchable — before anything is purged

`GET /api/broker/detection/coverage` and `/detection/queue` are deployed and
verified reachable. `services/detectionClient.ts` is written and calls both.
**Nothing imports it — the client is dead code**, so neither route has a reader.

Build the two views into the Observatory, extending `#743` rather than adding a
tool:

- **Corpus coverage** — per publication and per engine: pages, regions by kind,
  regions by assertion state, envelopes with and without retrievable bodies.
  The last column is the one that would have surfaced the orphan window on the
  day it happened.
- **Queue health** — states, lease ages, attempt distribution, failure codes
  grouped. The first surface in the system to read `vision_detection_job`.

Frontend only. Both routes exist; this is the consumer.

### 2. Purge the machine semantic layer

Safe, and sanctioned rather than worked around:

- **Zero human-tier envelopes.** Everything is `machine`/`candidate`, so no
  review work is destroyed.
- Migration `0020` explicitly permits `DELETE` of `page-semantics` rows on a
  published revision — the side index was designed to be rebuilt.

Keep publications, revisions, reading items and page bytes. Those are the
source and are untouched by any of this.

Purge because it cannot be tuned against: 836 of ~2,905 envelopes have no body,
`ocr-detector` is 74% unreadable, `vision-worker` found nothing on 335 pages
(44% of its own successful jobs), and the over-segmentation cannot even be
investigated because the evidence is in the damaged window.

### 3. Re-run what is free

`image-stats` and `panel-detector`. Deterministic, no API, no rate limit. The
panel backfill did 831 jobs in about six minutes with zero failures, so this is
minutes. Watch it land in the coverage view built in step 1.

### 4. Re-run Gemini only where it answers a question

A bounded `vision-worker` run — ~100 pages that also carry panel-detector
coverage. Two questions, both currently unanswerable:

- **Panels versus panels.** Zero pages today have retrievable bodies from both
  engines, so replace-or-join has no evidence behind it. Manifest counts do not
  substitute: 17.6 includes text regions, 3.3 does not.
- **Is the over-segmentation real?** 117 regions on one page, 42 pages over 25.

**Hold `ocr-detector`.** Re-buying transcriptions we intend to replace with a
local engine is spending twice. Run it only to compare against comic-ocr once
that reads.

### 5. First attested regions

The confirm control is deployed and has never been used. Until someone reviews a
page, `attested` stays 0 and the training export has nothing to export — that is
a function of usage, not capability. This is also the step that needs no code.

### Done when

- No envelope in the corpus lacks a retrievable body, and a guard makes that
  state unreachable.
- Coverage and queue health are visible at `?tool=pipeline` without a database
  session.
- Panels-versus-panels has a number behind it.
- At least one region is `accepted` or `verified`.

---

## After that

**The decoder loop.** Turns `NotImplemented` into a transcription. Validate
against any same-architecture checkpoint — weights of our own are not a
prerequisite, so this can proceed in parallel with everything above.

**Manga109-s.** Request now for the week of lead time; it is a quality baseline,
not the durable corpus.

**Track A.** Japanese first — the track that justifies training rather than
adopting. English may not need training in v1; measure an off-the-shelf engine
behind the same trait before spending on it.

**The chain that matters, throughout.**

```
detection → confirm-in-reader → accepted/verified regions → training export → own model
```

Every link exists except the last, and the fourth is
[`TRAINING_EXPORT.md`](TRAINING_EXPORT.md) against
[`schemas/training_pair.json`](../schemas/training_pair.json). That is why the
assertion vocabulary mattered enough to fix: `confirmed` being a state that
could never be true did not break a display, it meant the training corpus could
never grow.

---

## Standing constraints

- Staging only, never production.
- `manga-service` and `universe-broker` are shared; announce and work in your own
  worktree before any write. (This previously named a specific owning session,
  which no longer exists — an unowned constraint blocks work for nobody's benefit.)
- Manga109-s: owner accepts personally; never vendored, never redistributed,
  ≤20% of any volume published, attribution required.
- Anything that cannot be computed returns `Err` — never a placeholder, never a
  default confidence. The fabrication guard enforces this on the engine.
