# Exporting training pairs from the iPub graph

> **Status: design. None of this is built.** Verified 2026-08-20: zero
> occurrences of `export_pairs`, `ExportFilter`, `ExportReport` or `training_pair`
> across the 32 `.rs` files in this repository, and nothing references
> `schemas/training_pair.json`. The schema is the only artifact that exists.
>
> An earlier revision of this page described the filter, the report and the
> licence separation in the present tense, which read as though an exporter were
> running. It is not. Where the text below says the export "refuses" or
> "enforces", read it as *must*, not *does*.

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

## The one rule

**Only `accepted` and `verified` regions may be exported.** Not `candidate`,
which is a machine guess — exporting those trains a model to reproduce the engine
that produced them, including its mistakes, and that is a slower and less visible
version of no training at all. Not `rejected`, which was reviewed and thrown out;
exporting those trains the model on precisely the boxes a human said were wrong.

The schema encodes this as an enum of two values so a validator refuses the other
two rather than relying on the exporter to remember.

## Shape

```rust
pub struct ExportFilter {
    // NOTE: there is deliberately no `states` field. `accepted`/`verified` are
    // the only eligible states and that is not configurable — the point of the
    // gate is that it cannot be widened by a flag on a bad day. (The comment
    // asserting this was previously attached to `language`, describing a member
    // that does not exist.)
    pub language: Option<Language>,
    pub source: Source,
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
    pub skipped_body_unretrievable: usize,
}

pub fn export_pairs(
    graph: &SemanticSource,
    filter: &ExportFilter,
    out: &Path,
) -> Result<ExportReport, ExportError>;
```

`ExportReport` is the deliverable as much as the pairs are. An export that writes
400 pairs from 3,000 regions has made 2,600 decisions, and the run needs to say
what they were — otherwise a corpus that shrank because of a geometry bug looks
identical to one that shrank because the corpus is young.

## Crop derivation

1. Fetch page bytes via `reader-medium` → CAS.
2. Take `geometry.normalizedBounds`, already in `[0,1]`.
3. Multiply by the **stored** page dimensions — not the served image's, which may
   differ. Where they disagree, skip and count it rather than guessing.
4. Bounding box of the polygon, plus a small margin, clamped to the page.
5. Write PNG; the path goes in `crop`.

`geometry.normalized_bounds` is retained in the pair so crops can be regenerated
at another resolution without re-reviewing anything.

## Licence separation

`source` **must be** enforced at write time rather than documented and hoped for —
this is a requirement on the exporter, not a description of one:

- `manga109s` pairs are written to a **separate dataset root** from `own-corpus`.
  Redistribution of that data is forbidden, so the two must never share a
  directory that could be archived or copied as one unit.
- The export refuses to write a mixed manifest. A training run selects roots
  explicitly and records which it used.

## Where it runs

The export reads the platform's semantic envelopes, so it belongs on the platform
side, next to the training-set stage in the Observatory that already counts what
is eligible. That stage's `attested` figure is precisely this export's input
size — not a dashboard number, the corpus.

This repository consumes the output. It does not need to know how the graph is
shaped, only how to read `training_pair.json`.

## Current expected yield

**Zero.** No region in the corpus is `accepted` or `verified`; every one is
`machine`/`candidate`. The confirm control that produces attested regions is
deployed and has never been used.

Two independent reasons, and it is worth separating them because only one is
about usage:

1. **No region is attested.** The confirm control is deployed and has never been
   used, so the eligible set is empty.
2. **No exporter exists.** Nothing would read the regions even if they were
   attested.

An earlier version of this paragraph said "the exporter is correct, has nothing to
export" — which credits a program that was never written. Fixing (1) alone changes
nothing until (2) is built.
