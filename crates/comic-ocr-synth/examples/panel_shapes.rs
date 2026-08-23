//! Prove the guillotine layout is not degenerate before trusting any
//! measurement taken on it. A layout that collapses to slivers, or that
//! reproduces the grid it replaced, would still change the output hash.
use comic_ocr_synth::page::{PageSpec, render_page};
use comic_ocr_synth::render::SynthFont;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    let font = SynthFont::from_path(
        &std::env::var("COMIC_OCR_SYNTH_FONT")
            .unwrap_or_else(|_| "/System/Library/Fonts/Hiragino Sans GB.ttc".into()),
        0,
    )
    .expect("font");
    let texts: Vec<String> = ["そうだね", "ぎゃっ", "ウソでしょ"]
        .iter().map(|s| s.to_string()).collect();
    let irregular = std::env::var("IRREGULAR").is_ok();
    let base: u64 = std::env::var("SEED_BASE").ok().and_then(|v| v.parse().ok()).unwrap_or(4242);
    println!("irregular={irregular}");
    for seed in 0..4u64 {
        let mut rng = StdRng::seed_from_u64(base + seed);
        let ps = PageSpec {
            width: 1200, height: 1700, target_regions: 16,
            irregular_panels: irregular,
            panel_count: std::env::var("PANEL_COUNT").ok().and_then(|v| v.parse().ok())
                .unwrap_or(3 + (base.wrapping_add(seed) % 7) as u32),
            ..Default::default()
        };
        let Ok((_p, tp)) = render_page(&ps, &font, &texts, &mut rng) else { continue };
        let mut dims: Vec<String> = tp.panels.iter().map(|p| format!("{}x{}", p.width, p.height)).collect();
        dims.sort();
        let distinct: std::collections::HashSet<&String> = dims.iter().collect();
        let areas: Vec<u64> = tp.panels.iter().map(|p| p.width as u64 * p.height as u64).collect();
        let (mn, mx) = (areas.iter().min().copied().unwrap_or(0), areas.iter().max().copied().unwrap_or(0));
        println!("  seed {seed}: {} panels, {} distinct sizes, area range {}..{} (ratio {:.1}x)",
            tp.panels.len(), distinct.len(), mn, mx,
            if mn > 0 { mx as f64 / mn as f64 } else { 0.0 });
        println!("      {}", dims.join(" "));
    }
}
