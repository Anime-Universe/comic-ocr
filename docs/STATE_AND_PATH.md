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
| **OCR envelopes with no `blob_url`** | **300 of 406 — 74% unreadable** |
| Pages claiming >25 text regions | 42, worst is **117** (avg 12.9) |
| Regions `accepted` or `verified` | **0** |

### What that means

The transcription corpus is **not usable, and smaller than its headline**. Three
quarters of the envelopes record a digest and a count with no pointer to a body —
nothing can retrieve the text. Of the ~106 that remain, an unknown fraction are
over-segmented: 117 regions on one page is a balloon detector fragmenting, or
reading art as text.

Zero regions are attested, so there is no ground truth at all. The judge control
that produces it is deployed and unused.

---

## The blockers, in order

1. **Empty `blob_url` on 300 OCR envelopes.** Either the CAS upload failed and
   registration proceeded anyway, or the metadata write dropped the field. Fails
   silently because `transcription_count` still looks healthy. In
   `manga-service` — Management's repo, needs coordinating. **Blocking
   regardless of what else happens**, and blocking harder if more documents are
   ingested.
2. **Over-segmentation.** 117 regions on a page. `vision-worker` finds the
   regions and `ocr-detector` transcribes them, so this points at region
   detection, not transcription.
3. **No decoder loop.** The native path cannot produce text. Needs encoder run →
   decoder loop with KV cache → beam search (`num_beams: 4`,
   `length_penalty: 2.0`, `no_repeat_ngram_size: 3` — greedy is not equivalent)
   → detokenise.
4. **No weights.** Track A training has not started.
5. **No ground truth.** Nothing attested; the confirm loop has never been used.

---

## The path

**Now — fix what corrupts data.**
Chase the empty `blob_url` and the over-segmentation *before* ingesting more
documents. More volume through this pipeline produces more unretrievable
envelopes and more fragmented regions: more rows, not more data.

**Next — make one engine actually read.**
The decoder loop, validated against whatever checkpoint is available. This is
what turns `NotImplemented` into a transcription and makes everything downstream
measurable.

**Then — ground truth.**
Request Manga109-s now (a week's lead time, costs nothing to hold) for a quality
baseline. Meanwhile the confirm control is the only source of labels that are
ours; the `attested` count is not a dashboard number, it is the size of the
training corpus.

**Then — Track A.**
Japanese first: it is the track that justifies training rather than adopting.
English may not need training in v1 at all — measure an off-the-shelf engine
behind the same trait before spending the budget.

**Throughout — the chain that matters.**

```
detection → confirm-in-reader → accepted/verified regions → training export → own model
```

Every link exists except the last, and the fourth is the training-set stage in
the Observatory. That is why the assertion vocabulary mattered enough to fix:
`confirmed` being a state that could never be true did not break a display, it
meant the training corpus could never grow.

---

## Standing constraints

- Staging only, never production.
- `manga-service` and `universe-broker` are owned by another session; coordinate
  before any write.
- Manga109-s: owner accepts personally; never vendored, never redistributed,
  ≤20% of any volume published, attribution required.
- Anything that cannot be computed returns `Err` — never a placeholder, never a
  default confidence. The fabrication guard enforces this on the engine.
