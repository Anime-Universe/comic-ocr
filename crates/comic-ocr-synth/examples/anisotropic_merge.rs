//! Does a direction-aware merge separate what one radius cannot?
//!
//! Merging is the detector's failure (`how_many_share_a_box`): at 60 regions
//! only 42 candidates survive for 60 truths, and a single dilation radius
//! cannot fix it because the SAME radius must join glyphs inside one enclosure
//! and not bridge to the next.
//!
//! But those two jobs sit on different axes. Text flows along one direction, so
//! glyphs of one block are close ALONG it, while a neighbouring block is
//! separated ACROSS it. A square kernel cannot express that; a rectangular one
//! can.
//!
//! This measures a separable max-filter (dilation) with independent x and y
//! radii against the isotropic baseline. It changes no library code.
use comic_ocr_synth::detect::{Box2, DetectSpec, score};
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use image::{GrayImage, Luma};
use imageproc::contrast::{ThresholdType, otsu_level, threshold};
use imageproc::region_labelling::{Connectivity, connected_components};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Separable dilation: horizontal pass then vertical, radii independent.
fn dilate_rect(src: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    let (w, h) = src.dimensions();
    let mut mid = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let lo = x.saturating_sub(rx);
            let hi = (x + rx).min(w - 1);
            let mut m = 0u8;
            for xx in lo..=hi {
                m = m.max(src.get_pixel(xx, y).0[0]);
            }
            mid.put_pixel(x, y, Luma([m]));
        }
    }
    let mut out = GrayImage::new(w, h);
    for y in 0..h {
        let lo = y.saturating_sub(ry);
        let hi = (y + ry).min(h - 1);
        for x in 0..w {
            let mut m = 0u8;
            for yy in lo..=hi {
                m = m.max(mid.get_pixel(x, yy).0[0]);
            }
            out.put_pixel(x, y, Luma([m]));
        }
    }
    out
}

fn boxes_from(page: &GrayImage, rx: u32, ry: u32, spec: &DetectSpec) -> Vec<Box2> {
    let level = otsu_level(page);
    let binary = threshold(page, level, ThresholdType::BinaryInverted);
    let merged = dilate_rect(&binary, rx, ry);
    let labels = connected_components(&merged, Connectivity::Eight, Luma([0u8]));
    let max_label = labels.pixels().map(|p| p.0[0]).max().unwrap_or(0);
    let mut acc: Vec<Option<(u32, u32, u32, u32)>> = vec![None; max_label as usize + 1];
    for (x, y, px) in labels.enumerate_pixels() {
        let l = px.0[0] as usize;
        if l == 0 {
            continue;
        }
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
    let page_area = (page.width() * page.height()) as f32;
    let mut out = Vec::new();
    for (x0, y0, x1, y1) in acc.into_iter().flatten() {
        let b = Box2 {
            x: x0.saturating_add(rx).min(x1),
            y: y0.saturating_add(ry).min(y1),
            width: (x1.saturating_sub(x0)).saturating_sub(rx * 2).max(1),
            height: (y1.saturating_sub(y0)).saturating_sub(ry * 2).max(1),
        };
        if b.area() < spec.min_area || b.area() as f32 / page_area > spec.max_area_fraction {
            continue;
        }
        out.push(b);
    }
    let all = out.clone();
    out.retain(|b| {
        !all.iter().any(|o| {
            o != b
                && o.x >= b.x
                && o.y >= b.y
                && o.x + o.width <= b.x + b.width
                && o.y + o.height <= b.y + b.height
        })
    });
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
    // (rx, ry): the baseline is the square 6,6 the detector ships.
    let arms: [(u32, u32); 5] = [(6, 6), (3, 8), (8, 3), (2, 8), (4, 10)];
    print!("{:>7}", "truth");
    for (rx, ry) in arms {
        print!("{:>10}", format!("{rx}x{ry}"));
    }
    println!();
    println!("{}", "-".repeat(7 + 10 * arms.len()));

    for target in [8usize, 16, 24, 40, 60] {
        print!("{target:>7}");
        for (rx, ry) in arms {
            let mut recalls = Vec::new();
            for seed in 0..3u64 {
                let mut rng = StdRng::seed_from_u64(4242 + seed);
                let ps = PageSpec {
                    width: 1200,
                    height: 1700,
                    target_regions: target,
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
                let found = boxes_from(&page, rx, ry, &spec);
                recalls.push(score(&truth, &found, 0.5).recall);
            }
            let mean = recalls.iter().sum::<f32>() / recalls.len().max(1) as f32;
            print!("{:>9.1}%", 100.0 * mean);
        }
        println!();
    }
    println!();
    println!("6x6 is the shipping isotropic baseline. A rectangular kernel that beats");
    println!("it at high density would mean the two merge jobs really are separable.");
}
