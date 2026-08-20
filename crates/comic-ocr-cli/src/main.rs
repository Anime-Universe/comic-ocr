use clap::Parser;
use comic_ocr_core::OcrEngine;
use comic_ocr_ort::OrtEngine;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "comic-ocr",
    version,
    about = "High-performance Comic & Manga OCR CLI in Rust"
)]
struct Cli {
    /// Path to input image file
    #[arg(short, long)]
    image: PathBuf,

    /// Extract Furigana readings into bracket syntax `漢[かん]字[じ]`
    #[arg(long, default_value_t = false)]
    extract_furigana: bool,

    /// Output full JSON conforming to schema
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Force CPU execution
    #[arg(long, default_value_t = false)]
    force_cpu: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    tracing::info!("Loading image from {:?}", cli.image);
    let img = image::open(&cli.image)?;

    let engine = OrtEngine::new("kha-white/manga-ocr-base").with_furigana(cli.extract_furigana);

    let result = engine.predict(&img)?;

    if cli.json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{}", json_str);
    } else {
        println!("Recognized Text: {}", result.text);
        println!("Confidence: {:.4}", result.confidence);
        println!("Duration: {:.2} ms", result.metadata.duration_ms);
    }

    Ok(())
}
