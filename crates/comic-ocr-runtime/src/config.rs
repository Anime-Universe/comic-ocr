use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
    pub model_name: String,
    pub force_cpu: bool,
    pub max_batch_size: usize,
    pub pdp_invalidation_threshold: f32,
    pub log_level: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("MANGA_OCR_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("MANGA_OCR_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
            model_name: std::env::var("MANGA_OCR_MODEL")
                .unwrap_or_else(|_| "kha-white/comic-ocr-base".into()),
            force_cpu: std::env::var("MANGA_OCR_FORCE_CPU")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            max_batch_size: 16,
            pdp_invalidation_threshold: 0.70,
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        }
    }
}
