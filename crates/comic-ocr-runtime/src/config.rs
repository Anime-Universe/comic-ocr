use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
    pub model_name: String,
    /// Filesystem path to an ONNX model. When set, the runtime loads a native
    /// session and never touches the Python subprocess path.
    ///
    /// This is what makes the ONNX engine reachable from the service at all:
    /// `OrtEngine::new` builds an engine with `session: None`, so a runtime that
    /// only ever called it could take the subprocess path and nothing else —
    /// while the container ships no Python.
    pub onnx_model_path: Option<String>,
    pub force_cpu: bool,
    pub max_batch_size: usize,
    pub pdp_invalidation_threshold: f32,
    pub log_level: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("COMIC_OCR_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            // `PORT` first: Railway, Fly and Heroku all inject it, and a service
            // that binds its own default there is unreachable no matter how
            // healthy it reports itself to be.
            port: std::env::var("PORT")
                .or_else(|_| std::env::var("COMIC_OCR_PORT"))
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8000),
            // Deliberately NO default. This project trains its own weights; a
            // built-in fallback to somebody else's checkpoint is how a service
            // ends up quietly serving a model nobody chose — and in this case a
            // Japanese-only one, against a bilingual corpus. Empty means "not
            // configured", which the runtime reports rather than papers over.
            model_name: std::env::var("COMIC_OCR_MODEL").unwrap_or_default(),
            onnx_model_path: std::env::var("COMIC_OCR_ONNX_PATH")
                .ok()
                .filter(|path| !path.trim().is_empty()),
            force_cpu: std::env::var("COMIC_OCR_FORCE_CPU")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            max_batch_size: 16,
            pdp_invalidation_threshold: 0.70,
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        }
    }
}
