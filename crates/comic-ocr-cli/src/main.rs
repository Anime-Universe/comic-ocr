use clap::Parser;
use comic_ocr_core::OcrEngine;
use comic_ocr_ort::OrtEngine;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "comic-ocr",
    version,
    about = "High-performance Japanese Comic OCR CLI in Rust"
)]
struct Cli {
    /// Path to input image file
    #[arg(short, long)]
    image: PathBuf,

    /// Extract Furigana readings into bracket syntax `漢[かん]字[じ]`
    #[arg(long, default_value_t = false)]
    extract_furigana: bool,

    /// Force CPU execution
    #[arg(long, default_value_t = false)]
    force_cpu: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    tracing::info!("Loading image from {:?}", cli.image);
    let img = image::open(&cli.image)?;

    let engine = OrtEngine::new("kha-white/comic-ocr-base").with_furigana(cli.extract_furigana);

    let result = engine.predict(&img)?;

    println!("Recognized Text: {}", result.text);
    println!("Confidence: {:.4}", result.confidence);
    println!("Duration: {:.2} ms", result.metadata.duration_ms);

    Ok(())
}
