//! Use the enclosure outline as the separator instead of deleting it.
//!
//! Three refutations narrowed this: not a per-page radius, not the filters, not
//! a global kernel shape. All three were GLOBAL parameters, and merging is
//! LOCAL — two balloons merge because of how close they are, not how dense the
//! page is.
//!
//! A balloon is a closed outline. Two balloons are two outlines, however close.
//! That boundary survives at the UN-DILATED stage and is destroyed by the
//! dilation that merges glyphs — and then `drop_containers` deletes whatever
//! survived, which is the separator itself.
//!
//! This inverts it: find containers first, emit one region per container, and
//! merge only the glyphs that no container holds.
use comic_ocr_synth::detect::{Box2, DetectSpec, detect_regions, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use image::{GrayImage, Luma};
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use imageproc::region_labelling::{Connectivity, connected_components};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Component boxes, plus the label image so a pixel can be attributed to the
/// component it BELONGS to rather than merely the boxes it falls inside.
fn components_labelled(
    img: &GrayImage,
) -> (Vec<Box2>, Vec<u32>, Vec<u32>, image::ImageBuffer<Luma<u32>, Vec<u32>>) {
    let labels = connected_components(img, Connectivity::Eight, Luma([0u8]));
    let max = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);
    let mut acc: Vec<Option<(u32, u32, u32, u32)>> = vec![None; max as usize + 1];
    let mut ink: Vec<u32> = vec![0; max as usize + 1];
    for (x, y, px) in labels.enumerate_pixels() {
        let l = px.0[0] as usize;
        if l == 0 {
            continue;
        }
        ink[l] += 1;
        match &mut acc[l] {
            Some((x0, y0, x1, y1)) => {
                *x0 = (*x0).min(x);
                *y0 = (*y0).min(y);
                *x1 = (*x1).max(x);
                *y1 = (*y1).max(y);
            }
            slot => *slot = Some((x, y, x, y)),
        }
    }
    let mut boxes = Vec::new();
    let mut ids = Vec::new();
    let mut inks = Vec::new();
    for (i, e) in acc.into_iter().enumerate() {
        if let Some((x0, y0, x1, y1)) = e {
            boxes.push(Box2 {
                x: x0,
                y: y0,
                width: (x1 - x0).max(1),
                height: (y1 - y0).max(1),
            });
            ids.push(i as u32);
            inks.push(ink[i]);
        }
    }
    (boxes, ids, inks, labels)
}

fn components(img: &GrayImage) -> Vec<Box2> {
    let labels = connected_components(img, Connectivity::Eight, Luma([0u8]));
    let max = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);
    let mut acc: Vec<Option<(u32, u32, u32, u32)>> = vec![None; max as usize + 1];
    let mut ink: Vec<u32> = vec![0; max as usize + 1];
    for (x, y, px) in labels.enumerate_pixels() {
        let l = px.0[0] as usize;
        if l == 0 {
            continue;
        }
        ink[l] += 1;
        match &mut acc[l] {
            Some((x0, y0, x1, y1)) => {
                *x0 = (*x0).min(x);
                *y0 = (*y0).min(y);
                *x1 = (*x1).max(x);
                *y1 = (*y1).max(y);
            }
            slot => *slot = Some((x, y, x, y)),
        }
    }
    acc.into_iter()
        .flatten()
        .map(|(x0, y0, x1, y1)| Box2 {
            x: x0,
            y: y0,
            width: (x1 - x0).max(1),
            height: (y1 - y0).max(1),
        })
        .collect()
}

fn contains(outer: &Box2, inner: &Box2) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
        && (inner.width < outer.width || inner.height < outer.height)
}

/// Enclosure-first detection.
fn detect_enclosure_first(page: &GrayImage, spec: &DetectSpec) -> Vec<Box2> {
    let level = otsu_level(page);
    let binary = threshold(page, level, ThresholdType::BinaryInverted);
    let page_area = (page.width() * page.height()) as f32;

    // Un-dilated: outlines are intact and two balloons are two components.
    let (raw, raw_ids, raw_ink, raw_labels) = components_labelled(&binary);

    // A container holds at least one other component and is not the page frame.
    // A CJK glyph is a container of its own counter -- 口, 日, 目, 田 all enclose
    // a closed loop -- so "holds another component" alone identifies every such
    // character as an enclosure. Measured: 15 containers on a page with 8
    // balloons.
    //
    // What separates a balloon from a 口 is not that it encloses something, but
    // how much larger it is than what it encloses. A balloon dwarfs its glyphs;
    // a counter nearly fills its character.
    let enclosure_ratio: f32 = std::env::var("ENCLOSURE_RATIO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6.0);
    let mut containers: Vec<Box2> = raw
        .iter()
        .filter(|c| {
            // An enclosure has to be big enough to BE one. `min_area: 400`
            // admits a 21x21 box, which is a glyph counter -- 口, 日, 目 all
            // enclose a loop just over that threshold. Measured on a sparse
            // page: 9 emitted containers for 6 truth regions, and the three
            // extras were 21x21, 21x21, 20x21.
            //
            // This is a floor on the ENCLOSURE, distinct from the floor on a
            // region. The smallest real balloon here is about 50x70.
            // REFUTED as the fix for the sparse regime (measured 2026-08-23,
            // SEED_BASE 9001 and 31337, held out):
            //
            //   floor  base 9001 sparse   base 31337 sparse
            //   2000   95.8 / 76.7        87.5 / 70.5
            //   3200   95.8 / 76.7        87.5 / 70.5
            //
            // Byte-identical. No container on either held-out corpus has an
            // area between 2000 and 3200, so the floor is NOT BINDING off the
            // corpus it was tuned on. On 4242 it does bind, and raising it
            // costs a real region: recall 95.8 -> 91.7 to buy precision
            // 72.1 -> 68.8. Worse on both axes.
            //
            // So the floor either does nothing or removes a true region. The
            // sparse false positives are larger than this threshold reaches,
            // which is consistent with them being enclosures SPLIT in two
            // rather than sub-threshold noise admitted. A broken outline
            // yields two partial containers, each holding some of the text --
            // and both survive any floor that keeps the smallest real balloon.
            //
            // Note also what the identical rows say about this parameter: it
            // was tuned on 4242 and is inert everywhere else measured.
            let enclosure_min: u32 = std::env::var("ENCLOSURE_MIN_AREA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000);
            if (c.area() as f32) / page_area > spec.max_area_fraction || c.area() < enclosure_min {
                return false;
            }
            let held: Vec<&Box2> = raw.iter().filter(|o| contains(c, o)).collect();
            if held.is_empty() {
                return false;
            }
            let largest = held.iter().map(|o| o.area()).max().unwrap_or(1).max(1);
            (c.area() as f32) / (largest as f32) >= enclosure_ratio
        })
        .cloned()
        .collect();
    // A panel frame contains balloons, which contain glyphs. Keep the innermost
    // containers -- the leaves of the containment forest, not its roots.
    // Keep only the INNERMOST enclosures. A panel frame contains balloons and
    // is not itself a region; a balloon contains only glyphs and is.
    //
    // The first version tested nesting against the already-filtered container
    // list, which never fired: a balloon holds glyphs below `min_area`, so no
    // balloon qualified as "contains a container", so no panel was ever
    // dropped. It emitted four identical 563x388 boxes -- the panel frames --
    // on every page, which was most of the precision loss.
    // MASK by every enclosure, EMIT only the innermost. Those are different
    // jobs and conflating them was the second bug here: dropping panel frames
    // from the emit list also dropped them from the mask, so panel stroke ink
    // became "loose", merged, and produced roughly one spurious box per frame.
    // The label of every enclosure, so its OWN ink is suppressed. Masking by
    // bounding box cannot do this: a panel's stroke lies inside the panel's
    // box, so it survived as "loose", merged, and produced a spurious box per
    // frame. A pixel belongs to exactly one component; that is the right unit.
    let mut mask_labels: std::collections::HashSet<u32> = raw
        .iter()
        .zip(raw_ids.iter())
        .filter(|(b, _)| containers.iter().any(|c| c == *b))
        .map(|(_, id)| *id)
        .collect();
    // An EMPTY enclosure is neither emitted nor masked, and that is the bug
    // the sparse regime exposes. `held.is_empty()` correctly refuses to EMIT a
    // panel that contains no region -- but it also drops that panel from the
    // MASK, so its own stroke stays "loose", dilates, and is emitted as a
    // panel-sized box from the merge path. Measured at 8 regions: 13 of 13
    // isolated false positives were 575x400 at panel-grid coordinates.
    //
    // This is the same MASK-vs-EMIT conflation recorded above, one level
    // deeper: there the emit list was used as the mask; here the mask is
    // derived from a filter whose job is to decide what to EMIT.
    //
    // An enclosure that holds nothing cannot be recognised by what it holds.
    // It is recognised by being an OUTLINE: large, and mostly not ink. A
    // 575x400 panel frame is ~1.7% ink; a text blob of the same bounds is far
    // denser.
    // Default ON: an enclosure that holds nothing being neither emitted nor
    // masked is a defect on its own terms. MASK_EMPTY=0 disables it.
    //
    // BUT THE NUMBER IS SYNTHETIC. Measured gains (recall unchanged on every
    // corpus; sparse precision):
    //   4242  72.1 -> 95.8     9001  76.7 -> 95.8
    //   31337 70.5 -> 87.5      777  65.8 -> 84.3
    // Those describe GENERATED pages. The failure signature there is identical
    // 575x400 boxes on a regular lattice -- an artefact of how pages are
    // synthesised, not a property of manga.
    //
    // On real pages the mechanism does not reproduce. Running the same
    // predicate (large, and under 10% ink) over the page-sized examples:
    // ZERO large-and-thin boxes on 5 of 5. A peer's independent count over 29
    // real sparse staging pages found no per-page surplus either -- the panel
    // detector emits exactly one panel on 18 of those 29.
    //
    // n=5 real pages is thin, and counts cannot see a SUBSTITUTION, only a
    // surplus: one empty frame replacing one real panel reads as pd=1 and is
    // invisible to both measurements. So this is "not observed on real pages",
    // not "cannot happen there".
    //
    // Do not quote the sparse-precision figures as staging performance.
    let mask_empty = std::env::var("MASK_EMPTY").map(|v| v != "0").unwrap_or(true);
    let outline_max_density: f32 = std::env::var("OUTLINE_MAX_DENSITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.10);
    let mut masks = containers.clone();
    if mask_empty {
        for ((b, id), ink) in raw.iter().zip(raw_ids.iter()).zip(raw_ink.iter()) {
            let a = b.area() as f32;
            if b.area() < 2000 || a / page_area > spec.max_area_fraction {
                continue;
            }
            if (*ink as f32) / a >= outline_max_density {
                continue;
            }
            if !mask_labels.contains(id) {
                mask_labels.insert(*id);
                masks.push(b.clone());
            }
        }
    }
    containers.retain(|c| !masks.iter().any(|o| o != c && contains(c, o)));

    // Glyphs no container holds: merge those the old way.
    let mut loose = GrayImage::new(page.width(), page.height());
    for (x, y, px) in binary.enumerate_pixels() {
        let p = Box2 {
            x,
            y,
            width: 1,
            height: 1,
        };
        // A pixel belongs to the SMALLEST enclosure containing it, and only
        // that one may suppress it. Suppressing by any enclosure removes free
        // text that merely sits inside a panel; suppressing by none lets panel
        // and balloon strokes merge into spurious boxes. Both were tried and
        // both lost — the first cost recall, the second precision.
        let innermost = masks
            .iter()
            .filter(|c| contains(c, &p))
            .min_by_key(|c| c.area());
        let own_label = raw_labels.get_pixel(x, y).0[0];
        let suppressed = mask_labels.contains(&own_label)
            || match innermost {
                // Inside an emitted enclosure: its box already represents this.
                Some(c) => containers.iter().any(|e| e == c),
                None => false,
            };
        if px.0[0] > 0 && !suppressed {
            loose.put_pixel(x, y, Luma([255]));
        }
    }
    let merged = dilate(&loose, Norm::LInf, spec.merge_radius);
    if std::env::var("DUMP_CONTAINERS").is_ok() {
        let mut sizes: Vec<(u32, u32, u32)> = containers
            .iter()
            .map(|c| (c.width, c.height, c.area()))
            .collect();
        sizes.sort_by_key(|s| std::cmp::Reverse(s.2));
        eprintln!(
            "    containers ({}): {:?}",
            sizes.len(),
            sizes
                .iter()
                .map(|(w, h, _)| format!("{w}x{h}"))
                .collect::<Vec<_>>()
        );
    }
    if std::env::var("SPLIT_SOURCES").is_ok() {
        let loose_n = components(&merged)
            .iter()
            .filter(|b| {
                b.area() >= spec.min_area && (b.area() as f32) / page_area <= spec.max_area_fraction
            })
            .count();
        eprintln!("    containers={} loose={}", containers.len(), loose_n);
    }
    let mut out = containers;
    for b in components(&merged) {
        if b.area() >= spec.min_area && (b.area() as f32) / page_area <= spec.max_area_fraction {
            out.push(b);
        }
    }
    out
}

fn main() {
    let font = SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    )
    .expect("font");
    let texts: Vec<String> = [
        "そうだね",
        "ちょっとまって",
        "ウソでしょ",
        "また迷路だし",
        "ぎゃっ",
        "少し黙っている",
        "実戦剣術も一流です",
        "素直にあやまるしか",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let spec = DetectSpec::default();
    // A FIGURE WITHOUT A CORPUS IDENTIFIER IS NOT QUOTABLE. Layout mode,
    // panel count and seed base determine the page geometry completely, so a
    // number measured under one is not comparable to a number under another.
    // The grid produced ONE panel size -- 564x389 on every page and every
    // seed -- so "held out across four corpora" meant four samples of one
    // geometry: the seeds varied text and placement, never layout.
    //
    // Printing the identifier with the table means a pasted number carries
    // its provenance, rather than relying on whoever pastes it to remember.
    {
        let base_id: u64 = std::env::var("SEED_BASE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4242);
        let layout = if std::env::var("IRREGULAR").is_ok() {
            "guillotine"
        } else {
            "grid(4x2)"
        };
        let pc = std::env::var("PANEL_COUNT").unwrap_or_else(|_| "varies 3-6".into());
        println!(
            "corpus: layout={layout} panels={pc} seed_base={base_id} mask_empty={}",
            std::env::var("MASK_EMPTY").map(|v| v != "0").unwrap_or(true)
        );
        println!("  (these figures are SYNTHETIC; no real-page ground truth exists — #833)");
    }
    println!(
        "{:>7} {:>10} {:>10} {:>6} {:>10} {:>10} {:>6}",
        "truth", "base-rec", "base-prec", "found", "enc-rec", "enc-prec", "found"
    );
    println!("{}", "-".repeat(64));
    let (mut b_sum, mut e_sum, mut n) = (0.0f32, 0.0f32, 0);
    let (mut bp_sum, mut ep_sum) = (0.0f32, 0.0f32);
    for target in [8usize, 16, 24, 40, 60] {
        let (mut br, mut er) = (Vec::new(), Vec::new());
        for seed in 0..3u64 {
            // SEED_BASE lets the threshold be validated on pages it was not
            // tuned on. A parameter fitted and measured on one sample is a
            // description of that sample.
            let base: u64 = std::env::var("SEED_BASE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4242);
            let mut rng = StdRng::seed_from_u64(base + seed);
            let irregular = std::env::var("IRREGULAR").is_ok();
            let ps = PageSpec {
                width: 1200,
                height: 1700,
                target_regions: target,
                irregular_panels: irregular,
                // Vary the panel count too -- a fixed count is the same
                // regularity one level up from a fixed grid.
                // PANEL_COUNT pins the count so regularity can be varied
                // alone. Irregular pages otherwise carry 3-6 panels against
                // the grid's 8, and fewer panels means less crowding -- a
                // comparison has to match conditions, not just code.
                panel_count: std::env::var("PANEL_COUNT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3 + (base.wrapping_add(seed) % 7) as u32),
                ..Default::default()
            };
            let Ok((page, tp)) = render_page(&ps, &font, &texts, &mut rng) else {
                continue;
            };
            let truth: Vec<Box2> = tp
                .regions
                .iter()
                .map(|r| {
                    let (x, y, w, h) = r.enclosure_bounds();
                    Box2 {
                        x,
                        y,
                        width: w,
                        height: h,
                    }
                })
                .collect();
            let b = detect_regions(&page, &spec);
            let e = detect_enclosure_first(&page, &spec);
            let bs = score(&truth, &b, 0.5);
            let es = score(&truth, &e, 0.5);
            br.push((bs.recall, bs.precision, b.len() as f32));
            er.push((es.recall, es.precision, e.len() as f32));
        }
        let k = br.len().max(1) as f32;
        let (bm, bp, bf) = (
            br.iter().map(|x| x.0).sum::<f32>() / k,
            br.iter().map(|x| x.1).sum::<f32>() / k,
            br.iter().map(|x| x.2).sum::<f32>() / k,
        );
        let (em, ep, ef) = (
            er.iter().map(|x| x.0).sum::<f32>() / k,
            er.iter().map(|x| x.1).sum::<f32>() / k,
            er.iter().map(|x| x.2).sum::<f32>() / k,
        );
        b_sum += bm;
        e_sum += em;
        bp_sum += bp;
        ep_sum += ep;
        n += 1;
        println!(
            "{:>7} {:>9.1}% {:>9.1}% {:>6.0} {:>9.1}% {:>9.1}% {:>6.0}",
            target,
            100.0 * bm,
            100.0 * bp,
            bf,
            100.0 * em,
            100.0 * ep,
            ef
        );
    }
    println!("{}", "-".repeat(64));
    println!(
        "{:>7} {:>9.1}% {:>9.1}% {:>6} {:>9.1}% {:>9.1}%",
        "mean",
        100.0 * b_sum / n as f32,
        100.0 * bp_sum / n as f32,
        "",
        100.0 * e_sum / n as f32,
        100.0 * ep_sum / n as f32
    );
    println!();
    println!("Recall alone is not a result: a detector that emits everything scores 100%.");
}
