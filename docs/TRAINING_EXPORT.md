# Exporting training pairs from the iPub graph

> **Status: library and CLI boundary implemented; platform orchestration not
> yet wired.** `comic-ocr-core::export_pairs` writes real PNG crops,
> JCS-canonical records, rejection telemetry, and a SHA-256-addressed immutable
> dataset manifest from one already-resolved page. The caller must supply the
> platform-issued rights grant and split assignment. Resolving those records
> from the live platform graph and validating the active grant remains the
> platform compiler's responsibility.

How `accepted`/`verified` regions become `(crop, label)` pairs. Schema:
[`schemas/training_pair.json`](../schemas/training_pair.json).

## Why this is not `ocr_result.json`

`ocr_result.json` requires `text`, `confidence`, `token_probabilities` and
`metadata{duration_ms, model_name, engine_type}`. Every one of those describes an
inference *run*. A training pair has no model behind it — confidence and token
probabilities are not merely unknown, they are category errors, and filling them
is how `confidence: 0.985` ended up across 150KB of fixtures that no model ever
produced.

Two schemas, two lifecycle stages. The export writes pairs; the engine writes
results; nothing converts one into the other.

## The source

Everything needed is already in the graph. No new capture step:

```
publication_revision
  └── reading_item                     the page
        ├── reading_item_resource role='reader-medium'   → page bytes in CAS
        └── reading_item_resource role='page-semantics'  → envelope in CAS
              └── regions[]            id, kind, geometry.normalizedBounds, state
              └── textLayers[].regions[]  regionId → text, direction
```

The label is `textLayers[].regions[].text`. The crop is
`regions[].geometry.normalizedBounds` applied to the page bytes. The gate is
`regions[].state`.

## The Training Contract

Training eligibility requires both label evidence and an affirmative
training-purpose grant. Read or ingest entitlement alone is not a training
grant. Rights enforcement belongs to the platform compiler because this
page-scoped library does not have access to the rights ledger.

Under **The Training Contract**:
1. **Gold is the default**: the exporter includes only accepted and verified
   rows; they require canonical publication, item, page, envelope and reviewer
   provenance.
2. **Silver is explicit**: candidates may be exported only through
   `dataset_class: silver`. They remain machine-labelled, confidence-weighted
   silver data and must not be represented as human-attested gold data.
3. **Evaluation is isolated**: only independently `verified` assertions may use
   `dataset_class: evaluation`, and evaluation always maps to the `test` split.
   Silver and gold may use only `train` or `validation`.
4. **`rejected` is strictly excluded**: Regions explicitly invalidated by human review are wrong labels and remain excluded from export.
5. **Hierarchy of Confidence**:
   - `machine` (single uncorroborated engine): Engine's own confidence ($0.0 - 0.98$).
   - `corroborated` (two independent engines agree): Raised confidence.
   - `contested` (two engines disagree): Lowered confidence (held out in review queue).
   - `glanced` (human approved page at a glance): $0.5$.
   - `examined` (human confirmed region itself): $1.0$.

## Shape

```rust
pub struct ExportFilter {
    pub min_confidence: f32, // Select threshold, e.g. >= 0.5
    pub dataset_class: DatasetClass, // silver | gold | evaluation
    pub language: Option<String>,
    /// Skip regions whose crop would be smaller than the encoder input, which
    /// upsample into blur and teach nothing.
    pub min_crop_px: u32,
}

pub struct ExportReport {
    pub pairs_written: usize,
    /// Counted, never silently dropped — a filtered corpus that does not say
    /// what it filtered is indistinguishable from a small one.
    pub skipped_candidate: usize,
    pub skipped_rejected: usize,
    pub skipped_too_small: usize,
    pub skipped_no_geometry: usize,
    pub skipped_low_confidence: usize,
    pub skipped_missing_confidence: usize,
    pub skipped_empty_label: usize,
    pub skipped_wrong_class: usize,
}

pub fn export_pairs(
    context: &ExportContext,
    page: &DynamicImage,
    text_layers: &[TextLayer],
    filter: &ExportFilter,
    out: &Path,
) -> Result<ExportReport, String>;
```

`ExportReport` is the deliverable as much as the pairs are. An export that writes
400 pairs from 3,000 regions has made 2,600 decisions, and the run needs to say
what they were — otherwise a corpus that shrank because of a geometry bug looks
identical to one that shrank because the corpus is young. It is embedded in
`dataset-manifest.json` and also written separately for inspection.

## Immutable manifest and split discipline

Every successful page export writes [`schemas/dataset_manifest.json`](../schemas/dataset_manifest.json).
Each pair also carries the immutable `page_digest` alongside `envelope_digest`
and `region_id`, so its crop can be traced to exact page bytes rather than only
to a semantic assertion. Exporter tests validate the serialized silver, gold,
and evaluation records against `training_pair.json`, and validate their dataset
manifests against `dataset_manifest.json`; struct/schema drift is a failing gate.
The manifest records the policy version, class, split, lineage group, source,
rights-grant id, publication/item/page/envelope identities, reviewer, each
record and crop digest, and the complete rejection report. Records and the
manifest are canonicalized with RFC 8785 JCS before hashing; the manifest digest
is SHA-256 over the same object with an empty `manifest_digest` field.

`split_group` is required and must be assigned at work/publication lineage
scope by the platform compiler. That compiler must reject a group already
assigned to another split and reject repeated crop digests across manifests.
The page exporter detects duplicate crop digests within its own output; it does
not claim visibility into other export roots.

## Crop derivation

The page-scoped exporter implements steps 2–6. The platform compiler owns
step 1 and must prove that the supplied page and envelope are the immutable
resources named in `ExportContext`.

1. Fetch page bytes via `reader-medium` → CAS.
2. Take the text reading's normalized rectangle, already in `[0,1]`.
3. Multiply by the **stored** page dimensions — not the served image's, which may
   differ. Where they disagree, skip and count it rather than guessing.
4. Validate the rectangle and reject zero-area or sub-minimum crops.
5. Write PNG; the path goes in `crop`.
6. Hash the crop and canonical record and bind both into the immutable manifest.

`geometry.normalized_bounds` is retained in the pair so crops can be regenerated
at another resolution without re-reviewing anything.

## Licence separation

`source` separation is a requirement on the future platform compiler, not a
capability of the page-scoped exporter:

- `manga109s` pairs are written to a **separate dataset root** from `own-corpus`.
  Redistribution of that data is forbidden, so the two must never share a
  directory that could be archived or copied as one unit.
- The compiler refuses to write a mixed manifest. A training run selects roots
  explicitly and records which it used.

## Where it runs

The platform compiler belongs next to the training-set stage in the
Observatory. It resolves CAS resources, verifies that `semantic_training_grant`
is active and appropriate for training or evaluation, enforces corpus-wide
deduplication/split isolation, then calls the page-scoped exporter with explicit
immutable context.

This repository consumes the output. It does not need to know how the graph is
shaped, only how to read `training_pair.json`.

## Current expected yield

Unknown until the platform compiler queries the live corpus. Repository code
and fixtures cannot establish production attestation counts.
