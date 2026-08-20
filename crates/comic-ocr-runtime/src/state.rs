use crate::config::RuntimeConfig;
use comic_ocr_ort::OrtEngine;
use comic_ocr_pdp::PanelEvaluator;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct RuntimeMetrics {
    pub total_requests: AtomicU64,
    pub total_successful_ocr: AtomicU64,
    pub total_failed_ocr: AtomicU64,
    pub start_time: Instant,
}

pub struct RuntimeState {
    pub config: RuntimeConfig,
    pub engine: OrtEngine,
    pub pdp_evaluator: PanelEvaluator,
    pub metrics: RuntimeMetrics,
}

pub type SharedRuntimeState = Arc<RuntimeState>;

impl RuntimeState {
    pub fn new(config: RuntimeConfig) -> Self {
        let engine = OrtEngine::new(config.model_name.clone());
        let pdp_evaluator = PanelEvaluator::new(
            vec![Box::new(OrtEngine::new(config.model_name.clone()))],
            config.pdp_invalidation_threshold,
        );

        Self {
            config,
            engine,
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
