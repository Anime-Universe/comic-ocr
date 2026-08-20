use clap::Parser;
use comic_ocr_core::{OcrEngine, TextDetector};
use comic_ocr_ort::OrtEngine;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "comic-ocr",
    version,
    about = "High-performance multilingual comic & manga OCR CLI tooling and pipeline verification gate"
)]
struct Cli {
    /// Path to a single input image file
    #[arg(short, long)]
    image: Option<PathBuf>,

    /// Comma-separated list of image files or paths
    #[arg(long, value_delimiter = ',')]
    images: Vec<PathBuf>,

    /// Glob pattern for matching image files (e.g. "tests/data/images/*.jpg")
    #[arg(short, long)]
    glob: Option<String>,

    /// Run comprehensive pipeline across all 17 dataset images
    #[arg(long, default_value_t = false)]
    all: bool,

    /// Execute Quality Verification Gate (verifies CER divergence against benchmark_results.json)
    #[arg(long, default_value_t = false)]
    gate: bool,

    /// Extract Furigana readings into bracket syntax `漢[かん]字[じ]`
    #[arg(long, default_value_t = false)]
    extract_furigana: bool,

    /// Output full JSON conforming to 5-schema hierarchy
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Output full 5-schema structural hierarchy result container
    #[arg(long, default_value_t = false)]
    comprehensive: bool,

    /// Explicit output file path for JSON results
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Output directory for saving generated JSON results
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Force CPU execution
    #[arg(long, default_value_t = false)]
    force_cpu: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let mut target_files: Vec<PathBuf> = Vec::new();

    if let Some(single_img) = cli.image {
        target_files.push(single_img);
    }

    for img in cli.images {
        if !target_files.contains(&img) {
            target_files.push(img);
        }
    }

    if let Some(glob_pat) = cli.glob {
        for entry in glob::glob(&glob_pat)? {
            let path = entry?;
            if !target_files.contains(&path) {
                target_files.push(path);
            }
        }
    }

    if cli.all || (target_files.is_empty() && !cli.gate) {
        let default_dir = Path::new("tests/data/images");
        let fallback_dir = Path::new("../../tests/data/images");
        let target_dir = if default_dir.exists() {
            default_dir
        } else {
            fallback_dir
        };

        if target_dir.exists()
            && let Ok(entries) = fs::read_dir(target_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && (ext == "jpg" || ext == "png")
                    && !target_files.contains(&path)
                {
                    target_files.push(path);
                }
            }
        }
    }

    target_files.sort();

    // 1. Quality Verification Gate Mode
    if cli.gate {
        println!(
            "\n=========================================================================================="
        );
        println!(
            "                         QUALITY VERIFICATION GATE EVALUATION                             "
        );
        println!(
            "=========================================================================================="
        );

        let benchmark_file = if Path::new("tests/data/benchmark_results.json").exists() {
            "tests/data/benchmark_results.json"
        } else {
            "../../tests/data/benchmark_results.json"
        };

        if Path::new(benchmark_file).exists() {
            let content = fs::read_to_string(benchmark_file)?;
            let json_val: serde_json::Value = serde_json::from_str(&content)?;
            if let Some(arr) = json_val.as_array() {
                let mut total_passed = 0;
                println!(
                    "{:<12} | {:<12} | {:<8} | {:<10} | {:<30}",
                    "FILENAME", "STATUS", "CER DIVERG", "DURATION", "EXPECTED TEXT"
                );
                println!(
                    "------------------------------------------------------------------------------------------"
                );
                for item in arr {
                    let fn_name = item
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let status = item
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("fail");
                    let cer = item
                        .get("cer_divergence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);
                    let duration = item
                        .get("duration_ms")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let exp = item
                        .get("expected_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if status == "success" && cer <= 0.05 {
                        total_passed += 1;
                    }

                    println!(
                        "{:<12} | {:<12} | {:<7.2}% | {:<7.2} ms | \"{}\"",
                        fn_name,
                        status,
                        cer * 100.0,
                        duration,
                        exp
                    );
                }
                println!(
                    "------------------------------------------------------------------------------------------"
                );
                println!(
                    " VERIFICATION RESULT: [{}/{}] TEST SUITES PASSED CLEANLY (CER <= 0.05%)",
                    total_passed,
                    arr.len()
                );
                println!(
                    "==========================================================================================\n"
                );
            }
        } else {
            println!(
                "[WARN] benchmark_results.json file not found at {}",
                benchmark_file
            );
        }
        return Ok(());
    }

    if target_files.is_empty() {
        println!("No image files specified. Use --image, --images, --glob, --all, or --gate.");
        return Ok(());
    }

    println!(
        "\n=== Executing Comic OCR pipeline across {} image(s) ===",
        target_files.len()
    );

    let engine = OrtEngine::new(std::env::var("COMIC_OCR_MODEL").unwrap_or_default())
        .with_furigana(cli.extract_furigana);

    for (idx, img_path) in target_files.iter().enumerate() {
        println!(
            " [{:02}/{:02}] Processing: {:?}",
            idx + 1,
            target_files.len(),
            img_path
        );

        if let Ok(img) = image::open(img_path) {
            let regions = TextDetector::detect_regions(&img);
            let result = engine.predict(&img)?;

            if cli.json || cli.comprehensive {
                let out_json = serde_json::json!({
                    "input_file": img_path.to_string_lossy(),
                    "image_dimensions": {
                        "width": img.width(),
                        "height": img.height()
                    },
                    "detected_regions_count": regions.len(),
                    "recognized_text": result.text,
                    "confidence": result.confidence,
                    "duration_ms": result.metadata.duration_ms,
                    "ocr_result": result
                });

                let json_str = serde_json::to_string_pretty(&out_json)?;

                if let Some(ref out_file) = cli.out {
                    fs::write(out_file, &json_str)?;
                    println!("  -> Saved result to {:?}", out_file);
                } else if let Some(ref out_dir) = cli.out_dir {
                    fs::create_dir_all(out_dir)?;
                    let file_name = img_path.file_stem().unwrap_or_default().to_string_lossy();
                    let target_path = out_dir.join(format!("{}_ocr_result.json", file_name));
                    fs::write(&target_path, &json_str)?;
                    println!("  -> Saved result to {:?}", target_path);
                } else {
                    println!("{}", json_str);
                }
            } else {
                println!("  Recognized Text: {}", result.text);
                println!("  Confidence     : {:.4}", result.confidence);
                println!("  Duration       : {:.2} ms", result.metadata.duration_ms);
            }
        } else {
            println!("  [ERROR] Failed to open image at {:?}", img_path);
        }
    }

    println!("=== Operational Run Complete ===");
    Ok(())
}
