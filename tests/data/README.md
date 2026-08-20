# What these fixtures actually measure

Not everything in these files was measured. This note says which fields were,
because the ones that were not look exactly like the ones that were.

## Measured, and safe to rely on

| Field | Where it comes from |
| --- | --- |
| `filename`, `size_bytes` | the image files in `tests/data/images/` |
| `expected_text` | hand-authored ground truth — a person read the page |
| `actual_text` | a recorded model output |
| `cer_divergence` | Levenshtein CER between the two, and `test_benchmark_schema_integrity` re-computes it on every run and fails if the recorded value drifts by more than 1e-4 |

## Placeholders, despite appearances

| Field | Reality |
| --- | --- |
| `confidence` | Almost always exactly `0.985`. It was never computed. The engine used to hardcode it, and the generator scripts under `scripts/` still write it as a literal. |
| `token_probabilities` | Same — `[0.99, 0.985, 0.988]` and similar are literals, not softmax output. |
| `duration_ms` | Constant per file (`42.5` across all 17 rows of `benchmark_results.json`, `28.4` throughout `12_…`, `14.2` throughout `14_…`). A single identical duration across an entire benchmark is the tell: real timing varies. |

A whole benchmark reporting one confidence and one duration is the signature of
values that were written rather than observed.

## Why they are still here

`confidence` is `required` by `schemas/ocr_result.json` and
`schemas/pdp_decision.json`, so removing it invalidates the fixtures against
their own schemas. Rewriting the schemas to make it optional is the right change,
but the honest fix is to regenerate this data from a working engine — and no
engine can currently produce a transcription:

- the subprocess path needs `python3` + `torch` + `transformers`, which the
  shipped image does not carry;
- the native ONNX path loads a session and runs one forward pass, but
  VisionEncoderDecoder generation (decoder loop with KV cache) is not
  implemented, so it returns `OcrError::NotImplemented` rather than a
  placeholder.

## What to do when that lands

Regenerate these files by running the engine over `tests/data/images/`, and let
`confidence` and `token_probabilities` carry the real softmax values the engine
now computes. Both paths already derive them correctly — geometric mean of
per-step max probability — so the only missing piece is text.

Until then, `cargo test -- --ignored` runs
`test_benchmark_model_inference_evaluation`, which performs live inference and
asserts CER per image (≤ 0.20) and across the dataset (≤ 0.05). That test is the
one that will replace these numbers.
