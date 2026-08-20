use crate::config::RuntimeConfig;
use comic_ocr_ort::OrtEngine;
use comic_ocr_pdp::PanelEvaluator;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Which inference path this process actually has.
///
/// Reported on `/v1/runtime/health` because "the server is up" and "the server
/// can read text" are different facts, and a deployment that conflates them
/// looks healthy while failing every request. The container ships no Python, so
/// `Subprocess` in production means inference is unavailable — the runtime says
/// so rather than waiting for callers to discover it one 502 at a time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    /// A native ONNX session is loaded. Inference is available in-process.
    Onnx { model_path: String },
    /// No ONNX model was configured or it failed to load; the engine will shell
    /// out to `python3`. Viable locally, never in the shipped image.
    Subprocess { reason: String },
}

impl Backend {
    pub fn inference_available(&self) -> bool {
        matches!(self, Backend::Onnx { .. })
    }
}

pub struct RuntimeMetrics {
    pub total_requests: AtomicU64,
    pub total_successful_ocr: AtomicU64,
    pub total_failed_ocr: AtomicU64,
    pub start_time: Instant,
}

pub struct RuntimeState {
    pub config: RuntimeConfig,
    pub engine: OrtEngine,
    pub backend: Backend,
    pub pdp_evaluator: PanelEvaluator,
    pub metrics: RuntimeMetrics,
}

pub type SharedRuntimeState = Arc<RuntimeState>;

/// Build an engine, preferring a native ONNX session.
///
/// Returns the backend alongside the engine so the failure is a value the
/// service can publish rather than a log line nobody reads. A configured model
/// path that fails to load is NOT silently downgraded to the subprocess path
/// without saying which error caused it.
fn build_engine(config: &RuntimeConfig) -> (OrtEngine, Backend) {
    match config.onnx_model_path.as_deref() {
        Some(path) => match OrtEngine::from_onnx_file(path) {
            Ok(engine) => (
                engine,
                Backend::Onnx {
                    model_path: path.to_string(),
                },
            ),
            Err(error) => {
                tracing::error!(
                    model_path = %path,
                    error = %error,
                    "[comic-ocr] ONNX model configured but did not load; inference is UNAVAILABLE"
                );
                (
                    OrtEngine::new(config.model_name.clone()),
                    Backend::Subprocess {
                        reason: format!("ONNX model at {path} failed to load: {error}"),
                    },
                )
            }
        },
        None => (
            OrtEngine::new(config.model_name.clone()),
            Backend::Subprocess {
                reason: "COMIC_OCR_ONNX_PATH is not set".to_string(),
            },
        ),
    }
}

impl RuntimeState {
    pub fn new(config: RuntimeConfig) -> Self {
        let (engine, backend) = build_engine(&config);

        if backend.inference_available() {
            tracing::info!(backend = ?backend, "[comic-ocr] native ONNX session loaded");
        } else {
            tracing::warn!(
                backend = ?backend,
                "[comic-ocr] no native session; this image ships no python3, so \
                 /v1/ocr/predict will return errors until COMIC_OCR_ONNX_PATH points at a model"
            );
        }

        let pdp_evaluator = PanelEvaluator::new(
            vec![Box::new(build_engine(&config).0)],
            config.pdp_invalidation_threshold,
        );

        Self {
            config,
            engine,
            backend,
            pdp_evaluator,
            metrics: RuntimeMetrics {
                total_requests: AtomicU64::new(0),
                total_successful_ocr: AtomicU64::new(0),
                total_failed_ocr: AtomicU64::new(0),
                start_time: Instant::now(),
            },
        }
    }

    pub fn record_request(&self) {
        self.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_success(&self) {
        self.metrics
            .total_successful_ocr
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.metrics
            .total_failed_ocr
            .fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(path: Option<&str>) -> RuntimeConfig {
        RuntimeConfig {
            host: "0.0.0.0".into(),
            port: 8000,
            model_name: "test-model".into(),
            onnx_model_path: path.map(str::to_string),
            force_cpu: true,
            max_batch_size: 1,
            pdp_invalidation_threshold: 0.7,
            log_level: "info".into(),
        }
    }

    /// The default deployment posture must not claim inference it cannot do.
    #[test]
    fn no_model_path_reports_inference_unavailable() {
        let (_engine, backend) = build_engine(&config_with(None));
        assert!(!backend.inference_available());
        match backend {
            Backend::Subprocess { reason } => assert!(reason.contains("COMIC_OCR_ONNX_PATH")),
            other => panic!("expected subprocess backend, got {other:?}"),
        }
    }

    /// A configured-but-broken model is the dangerous case: it looks deliberate.
    /// It must degrade to an UNAVAILABLE backend that names the path, not to a
    /// silent subprocess fallback that reports the same thing as "not configured".
    #[test]
    fn a_model_path_that_does_not_load_says_which_path_failed() {
        let (_engine, backend) = build_engine(&config_with(Some("/nonexistent/model.onnx")));
        assert!(!backend.inference_available());
        match backend {
            Backend::Subprocess { reason } => {
                assert!(reason.contains("/nonexistent/model.onnx"), "got {reason}");
                assert!(reason.contains("failed to load"), "got {reason}");
            }
            other => panic!("expected subprocess backend, got {other:?}"),
        }
    }
}
