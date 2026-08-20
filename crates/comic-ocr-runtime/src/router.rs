use crate::handlers::{eval_panel_handler, health_handler, predict_handler, runtime_info_handler};
use crate::state::SharedRuntimeState;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn create_router(state: SharedRuntimeState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Core v1 API endpoints
        .route("/v1/runtime/health", get(health_handler))
        .route("/v1/runtime/info", get(runtime_info_handler))
        .route("/v1/ocr/predict", post(predict_handler))
        .route("/v1/ocr/eval_panel", post(eval_panel_handler))
        // Legacy fallback routes
        .route("/health", get(health_handler))
        .route("/ocr", post(predict_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
