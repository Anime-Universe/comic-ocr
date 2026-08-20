# Boundaries, and the geometry that produces them

Status: normative. Written 2026-08-20, against measured facts.

Companion to [`TRAINING_PATH.md`](TRAINING_PATH.md). That document covers the
text reader. This one covers what has to exist *before* the reader is called: how
a page is divided, what the division is recorded as, and why none of it is a crop.

**Text training remains the first item of work.** This document defines a second
path so it can be picked up deliberately, not so it competes.

---

## The principle

**A region is a boundary on a page, never a cropped file.**

The encoder's input is a tensor, so a resample happens somewhere. The rule is
about what is *recorded*, not what is computed:

- Region geometry is stored page-native, against `page.source.nativeSize`.
- A crop is a **transient view materialised for one inference and discarded** —
  `view(page, polygon) -> tensor`. It is never persisted and never the unit of
  record.

This is not tidiness. It buys three things a crop directory cannot:

1. **Masked sampling.** With a polygon rather than a rectangle, the view can fill
   outside the balloon path with the balloon's own interior value instead of
   neighbouring art. That is a direct attack on the 14% truncation tracked in
   Infinite-Verse#837 and on sound effects that bleed across panel borders.
2. **Re-derivation.** A better detector re-reads the same page. A crop directory
   would have to be regenerated, and the old crops are indistinguishable from
   the new ones once written.
3. **Reversibility.** `masks[]`, `artRegions[]`, and clean-plate repair all
   operate in page space. A crop severs them from the page they repair.

The schemas already committed to this. `localized_text_object.json` requires
`geometry.bounds` **and** `geometry.transform`; a transform only means anything
if the source pixels are still where they were.

---

## The schema layer has drifted, and nothing was checking

Measured 2026-08-20 by [`scripts/validate_schemas.py`](../scripts/validate_schemas.py):

```
4 of 6 examples fail their schema.
```

| Example | Conforms? | Nature of the drift |
| --- | --- | --- |
| `sample_ocr_result.json` | **yes** | — |
| `sample_ipub_semantic_resource.json` | **yes** | — |
| `sample_comic_scene_graph.json` | no | missing `textRegions`, `coordinateSystem`, and mask `geometry`/`fill` |
| `sample_page_result.json` | no | uses `page_id`/`reading_order`/`bounds[]`; schema requires `page_index`/`panel_index`/`bounding_box{}` |
| `sample_localized_text_object.json` | no | flat; schema requires seven nested groups |
| `sample_pdp_decision.json` | no | uses `engine_type`/`text`/`acs_score`; schema requires `engine_name`/`predicted_text`/`discounted_weight` |

The cause is structural, not careless. There is **no JSON Schema validator
anywhere in the build** — no `jsonschema`, `valico`, or `schemars` dependency —
and `crates/comic-ocr-core/tests/test_schema_json_suite.rs` parses four of the
six examples as untyped `serde_json::Value`, asserting on individual keys. It
passes against the *examples*, which is why it never noticed that the examples
stopped matching the *schemas*.

Only two examples are checked against real Rust types: `MangaDocument` and
`OcrResult`.

### The settlement

**1. `comic_scene_graph.json` is the normative document model.**

It is the only one of the three with typed Rust behind it
(`crates/comic-ocr-core/src/scene_graph.rs`), and it already has the shape the
pages demand: `textRegions` is a **sibling** of `panels` with an *optional*
`panelId`, `PanelFrame` and `ContainerGeometry` each carry
`polygon: Option<Polygon>` alongside `bounds`, and `Panel` carries `zIndex`.

**2. `page_result.json` is an engine-output projection, not a document model —
and its nesting is wrong.**

It nests `bubbles` inside `panels[]`, which forces every text region to have
exactly one panel parent. Real pages break this constantly: balloons floating in
white space between panels, balloons spanning a panel border, bubble chains with
no panels at all. Someone already hit the wall and cut an escape hatch —
`onomatopoeia_crops` is hoisted to the top level precisely because sound effects
would not fit the nesting.

The fix is to hoist text regions to page level with an *optional* panel
reference, matching the scene graph, and to fold `onomatopoeia_crops` back in as
`role: "sound-effect"` rather than a parallel array.

**3. `localized_text_object.json` is a promise with no symbols.**

The schema describes a compiled, post-translation, ready-to-render object —
hence `cleanup.maskIds` and `rendering.antialias`. **No Rust type implements it.**
Its example is a `TextRegion` wearing the wrong name, which is why all seven
required groups are absent.

It is not reconciled here, because reconciling it would mean choosing between
two different artifacts, and only one of them is real. Either implement the
compiled form or retire the schema — that is an owner's call, and it does not
block anything below.

**4. Add the validator as a gate — after the examples are fixed, not before.**

Turning `validate_schemas.py` into a test today makes the suite red on four
counts unrelated to any change in flight. The script exists now so the drift is
measurable and reproducible; promoting it to a gate is the last step of the
schema work, not the first.

### Geometry gaps in the normative model

Three places the scene graph cannot yet express a boundary:

| Type | Has | Missing |
| --- | --- | --- |
| `MaskRegion` (`scene_graph.rs:329`) | `expansion`, `feather`, `type` | **all geometry** — no bounds, no polygon |
| `ArtRegion` (`scene_graph.rs:318`) | `bounds: Option<DualRect>` | polygon; a protected face is not a rectangle |
| `Polygon` (`scene_graph.rs:26`) | `Vec<Point>` | **holes** — a ring with no interior rings |

The third matters most. `ImageTracer` already produces holes and already detects
them (`is_hole_path`, `point_in_poly`), and `RingsOutput` already carries
`{outer, holes}`. The scene graph flattens that away.

---

## The pipeline mostly exists

`Runtimes/compute/geometry-runtime` (v0.6.0, ~4,200 lines) was built for sticker
die-cutting and performs most of the proposed pipeline already.

| Stage | Status |
| --- | --- |
| Edge detection | **absent** — `imageproc = "0.25"` is a dependency and only `filter::gaussian_blur_f32` is imported; `edges::canny` is unused |
| Connected components | **absent as such** — the 4-way BFS flood fill exists (`mask_pipeline.rs:171`) but is not exposed as labelled components; `region_labelling::connected_components` is unused |
| Contour tracing | **present** — `ImageTracer::pathscan` → `trace_path`, with hole detection |
| Simplification | **present twice** — `douglas_peucker` (`tracer.rs:1183`) and `simplify` / `chaikin_smooth` (`vector_smooth.rs`) |
| Morphology / clean plate | **present** — dilate, erode, open, close, `remove_white_halo`, `expand_and_smooth` |
| Offsetting | **present** — `polygon_offset` on `clipper2-rust` |
| Classification | **absent — and it is the actual work** |

Everything named "absent" above is already vendored in the dependency tree:
imageproc 0.25.1 ships `edges::canny`, `region_labelling::connected_components`,
`contours::find_contours`, `contrast::otsu_level`, and
`contrast::adaptive_threshold`. None of it needs a new dependency.

The morphology set maps almost directly onto the scene graph's
`masks[].type: "clean-balloon" | "repair-art"`. The clean-plate step is not
future work; it is written and pointed at the wrong domain.

### ImageTracer does not work at all — measured, not predicted

This document originally parked "does the tracer survive screentone?" as an
unknown. It was measured on 2026-08-20 and the answer is prior to the question:
**`ImageTracer::trace` returns zero paths on every input.**

On `tests/data/images/12.jpg` (a 1024x793 two-page spread), with the shipped
code:

| Config | Layers | Paths |
| --- | --- | --- |
| `default()` (16 colours) | 16 | **0** |
| `contour_holes()` (the production sticker preset) | 2 | **0** |
| tuned B/W (2 colours, blur 5) | 2 | **0** |

The stages before path extraction are healthy: colour quantisation assigns
146,317 and 665,715 pixels to its two indices, and `layering_step` produces
10,082 valid path-start cells per layer. `pathscan` then walks all 10,082 —
and every resulting path has **zero points**. Maximum points across all of
them: 0.

Two defects, and they compound:

1. **`trace_path` initialises `dir = 0`.** `pathscan` starts walks only at
   codes 4 and 11, and `PATHSCAN_LOOKUP[4][0]` and `PATHSCAN_LOOKUP[11][0]` are
   both `[-1,-1,-1,-1]`. The loop's first act is to read that entry and break.
   Every walk terminates before pushing a single point. imagetracerjs starts
   these walks at direction 1.
2. **The `new_arr_val` write-back is missing**, which the code's own comment
   states: `// (we track visited; arr stays read-only)`. The `visited` array
   that replaced it is written but never read inside the loop. Without the
   write-back a saddle cell (code 5 or 10) takes the same branch on every
   visit, so the walk cannot resolve ambiguous crossings.

Applying both together — direction 1, and writing `entry[0]` back to the
previous cell — produces paths. (They were applied together; whether the
direction fix alone suffices was not isolated.)

**This is a live silent failure, not dead code.** `PathsGenerator::generate`
handles `find_primary_path` returning `None` with a fallback of
`[[0,0],[0,1],[1,1],[1,0],[0,0]]` — a one-unit square. So `/v1/geometry/trace`
does not error. It returns a well-formed `GeometryOutput`, with a canonical
hash and passing validations, whose die-cut path is a scaled 1px square.

### Even repaired, the tracer is the wrong front end

With both defects fixed, and counting only paths whose bounding box exceeds
400 px (below that is noise at page scale):

| Page | `default()` | tuned B/W | Otsu -> components -> contours |
| --- | --- | --- | --- |
| `12.jpg` 1024x793 spread | 446 (175 ms) | 224 (64 ms) | **98** (20 ms) |
| `13.jpg` 671x1024 | 521 (125 ms) | 429 (67 ms) | **145** (17 ms) |
| `14.jpg` 650x1024 | 456 (107 ms) | 135 (48 ms) | **62** (11 ms) |
| `18.jpg` 1704x2580 | 2,330 (761 ms) | 740 (322 ms) | **360** (71 ms) |

`12.jpg` carries roughly 12 panels and 13 balloons — about 25 regions. Every
column over-produces, but the threshold front end is consistently tightest by
2–6x and 5–10x faster.

The predicted path explosion is real; it was simply masked by a more basic
defect. Quantise-then-layer-per-colour is the wrong decomposition for a page
whose ink is bimodal.

**Decision: do not port `ImageTracer`.** Use imageproc's
`contrast::otsu_level` -> `contrast::threshold` -> `region_labelling::connected_components`
-> `contours::find_contours` as the front end. Keep the tracer's *downstream*
helpers, which are independent of the broken walk and are worth having:
`douglas_peucker`, `polygon_offset` (clipper2), `chaikin_smooth`,
`check_self_intersection`, hole-parent assignment, and the DXF emitters.

That is a smaller port than the one this document originally proposed, and it
deletes the `find_primary_path` problem rather than solving it.

### The one assumption that must go

`find_primary_path` (`tracer.rs:1028`) selects **the single largest non-hole
path**, and `GeometryOutput` carries one `rings: {outer, holes}`. That is the
sticker premise: one subject, one die-cut, one contour.

A comic page is *N* regions in a **hierarchy** with no primary. The port needs a
forest — `Vec<GeometryOutput>` with parent links — not a single root. Everything
downstream of that selection (offset, simplify, validate, DXF) is already
per-ring and does not change.

Containment depth in that forest **is** `panel.zIndex`, which is what resolves
inset panels: an inset whose ring is contained by another panel's ring is a
child, and its text is counted once, against the child.

### Extraction, not a service call

geometry-runtime is Axum behind a coordinator JWT with a 20 MiB
`IMAGE_MAX_BYTES` and base64 image transport. comic-ocr-rust is a library. Per
page HTTP round trips with base64 payloads, across a corpus of 167 archives, is a
great deal of bytes for pure computation.

`decode_base64_image` / `image_to_base64_png` is the **runtime's** boundary, not
the algorithm's. The algorithm should be extracted as a crate both can depend on.

---

## Classification, and getting labels without labelling

The scene graph already declares the label space: 10 `container.type` values and
14 `textRegion.role` values. It is a supervised problem with **zero training
data today**.

The features, however, fall out of the geometry for free:

| Feature | Separates |
| --- | --- |
| compactness (`perimeter² / area`) | round speech balloon vs rectangular caption box |
| vertices surviving Douglas–Peucker at fixed tolerance | jagged shout balloon (many) vs ellipse (few) |
| hole count | balloon with a tail gap, panel with an inset |
| interior luminance mean and variance | flat balloon interior vs sound effect sitting on art |
| containment depth | panel / inset / container / region |
| aspect ratio and absolute area | spine strip, gutter, full-bleed splash |

A small decision tree over those plausibly separates `speech-balloon`,
`caption-box`, `open-text`, and `sound-effect` before any neural classifier is
trained — and every one is computable from what `PathsGenerator` already emits.
That is the cheapest available route to labels we did not author.

### What this stage needs from training, and why it is second

Two trainable components live here, and both are *later* than the text reader:

**Region classifier.** Bootstrapped from the geometric features above, then
corrected. It needs the same held-out human-labelled set that
[`TRAINING_PATH.md`](TRAINING_PATH.md) demands, scored on **field accuracy**
rather than CER.

**Detector.** Tracked as Infinite-Verse#834 and blocked on #837. Scored on
**recall**, which is the number nothing currently measures: every failure mode
observed on real pages — unboxed sound effects, borderless full-bleed panels,
bubble chains without panels — is a way for text to exist and never be boxed.
We measure how well we read what we find and nothing about what we never found.

Both are second for a concrete reason: **3,004 real transcriptions exist and
zero region labels do**, and the corpus of conventionally-typeset publications is
large enough that the text reader can be trained and measured without any of
this. Geometry raises the ceiling; text training is what gets off the floor.

Matter pages (covers, colophons, credit pages, tables of contents) are a third
case and are deliberately excluded from the dialogue training set: they are the
cleanest printed text in any volume and would dominate a confidence-weighted
pool while teaching nothing about balloons.

---

## Order of work

1. **Text training** — unchanged, per [`TRAINING_PATH.md`](TRAINING_PATH.md).
   Nothing here precedes it.
2. **Fix the four drifted examples**, then promote `validate_schemas.py` to a
   test. Cheap, and it stops the layer drifting again.
3. **Hoist `page_result.json` text regions** to page level with an optional
   panel reference; fold `onomatopoeia_crops` in as a role.
4. **Give `MaskRegion` and `ArtRegion` geometry**, and give `Polygon` holes.
5. **Build the front end on imageproc** (Otsu -> components -> contours), not
   on `ImageTracer`; assemble the results into a containment forest. Take the
   tracer's simplification, offset, and hole-assignment helpers only.
6. **Feature-based classifier** over the forest; measure against a labelled set.
7. **Detector**, scored on recall (Infinite-Verse#834, blocked on #837).

## What is honestly still unknown

- Whether the geometric features actually separate the container types, or only
  look as though they should.
- Whether `LocalizedTextObject` should be implemented or retired.
