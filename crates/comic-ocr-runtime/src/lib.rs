pub mod config;
pub mod handlers;
pub mod router;
pub mod state;

pub use config::RuntimeConfig;
pub use router::create_router;
pub use state::{RuntimeState, SharedRuntimeState};

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_runtime_health_endpoint() {
        let config = RuntimeConfig::default();
        let state = Arc::new(RuntimeState::new(config));
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/runtime/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_runtime_info_endpoint() {
        let config = RuntimeConfig::default();
        let state = Arc::new(RuntimeState::new(config));
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/runtime/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
