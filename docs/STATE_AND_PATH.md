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

**The reference checkpoint is fully purged.** Zero `kha-white` references remain.

---

## Verified state — this repository

| Area | State |
| --- | --- |
| Build | compiles; **73 tests pass, 2 ignored**; `fmt` clean; `clippy --workspace --all-targets --all-features -D warnings` clean |
| Weights | **none exist.** No checkpoint on disk, none under any HF account, no training code in this repo (`comic_ocr_dev/training/` is referenced but absent; nearest is a reference project) |
| Native ONNX path | loads a session, runs one forward pass, computes real softmax confidence — then returns `OcrError::NotImplemented`, because VisionEncoderDecoder generation (decoder loop with KV cache) is not written |
| Subprocess path | works locally, needs `python3` + torch + transformers, which the shipped image does not carry |
| Tokenizer | **real and verified.** Vocab-file driven, derives special-token ids rather than assuming them. Differentially tested against the reference Python tokenizer: 304/304 exact match, both skip modes |
| Preprocessing | **real and measured.** 81–91% of tensor elements bit-identical to `ViTImageProcessor`, mean error 0.09–0.19 LSB. Constants marked as model properties to re-read per checkpoint |
| `resample_tiles` | **unwired** — exported and tested, no production caller. `max_aspect_ratio = 3.0` is provisional |
| Reading direction | fixed today; see below |
| Guards | fabrication scanner (4 tests) + CI (`fmt`, `clippy --all-targets`, `test`, `--no-run` so ignored tests can't rot) |

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

So this is **historical damage, not a live defect**. The shared
`register_envelope` path works — `panel-detector` uses it and is perfect. Most
likely blob-service or CAS was unreachable during that window while
registration proceeded regardless, recording a digest with no URL.

> **Correction.** This document previously said more documents should not be
> ingested until this was fixed. That was wrong, and it was wrong in the
> expensive direction — it would have halted ingestion over a scar rather than
> an open wound. Nothing is currently corrupting data.

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
3. **No ground truth.** Nothing attested. The confirm control is deployed and has
   never been used, so the training corpus is empty by usage, not by design.
4. **Over-segmentation, unexplained.** 117 text regions on one page, 42 pages
   over 25, average 12.9. `vision-worker` finds the regions and `ocr-detector`
   transcribes them, so this points at region detection. Cannot be diagnosed from
   stored data — the bodies for those pages are in the damaged window.

Note what is **not** on this list: the missing `blob_url`. It is historical, the
window closed on 08-19 01:00, and nothing is currently producing it.

---

## Two scoped jobs from the envelope damage

### A. Repair — 836 envelopes, but only some are worth it

| engine | missing | re-derivable? | worth re-running? |
| --- | ---: | --- | --- |
| `image-stats` | 394 | deterministic, free, no API | **yes** — pure compute, no reason not to |
| `vision-worker` | 142 | costs Gemini calls | **partly** — enough to enable the panels-vs-panels comparison |
| `ocr-detector` | 300 | costs Gemini calls | **probably not** — a local engine is coming; re-buying transcriptions we intend to replace is spending twice |

The re-run mechanism already exists and is proven: enqueue jobs with a fresh
idempotency key, as the panel-detector backfill did (831 jobs, zero failures,
about six minutes).

### B. Harden — one guard, in `manga-service`

The real defect is that **a failed upload was survivable**. `register_envelope`
recorded a digest with no `blob_url` and reported success, so
`transcription_count` still looked healthy while the body was unreachable.

The guard: refuse to register an envelope whose upload did not yield a URL —
return `Err`, let the job fail and retry, rather than persisting a row that reads
as success. Small, and it is what stops the next outage leaving the same scar.

Both are in Management's repo and need coordinating.

---

## The next big step

**A clean, measured detection baseline, watchable at
`https://stage.animeuniverse.com/?tool=pipeline`.**

That is the milestone. Everything currently measurable about detection is
either damaged, unreadable, or uncomparable, so no tuning decision can be made
from it. The step ends when the corpus has a baseline that can be trusted and
seen, which is also what makes the decoder loop's arrival measurable when it
lands.

### 0. The register guard — first, and blocking

A failed CAS upload is currently **survivable**: `register_envelope` records a
digest with no `blob_url` and reports success, so `transcription_count` reads
healthy while the body is unreachable. That is what produced 836 orphaned
envelopes between 08-18 11:00 and 08-19 00:00.

Fix: refuse to register an envelope whose upload did not yield a URL. Return
`Err`, let the job fail and retry, rather than persisting a row that reads as
success.

Owner: `manga-service` — another session. **Must be coordinated.** It is first
because a re-run without it can leave exactly the same scar, and then the whole
step is repeated.

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
- `manga-service` and `universe-broker` are owned by another session; coordinate
  before any write.
- Manga109-s: owner accepts personally; never vendored, never redistributed,
  ≤20% of any volume published, attribution required.
- Anything that cannot be computed returns `Err` — never a placeholder, never a
  default confidence. The fabrication guard enforces this on the engine.
