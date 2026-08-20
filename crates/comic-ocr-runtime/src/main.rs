use comic_ocr_runtime::{config::RuntimeConfig, create_router, state::RuntimeState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = RuntimeConfig::default();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.as_str().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("===========================================================");
    tracing::info!(
        "MANGA OCR REFLECTIVE RUNTIME v{}",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Model: {}", config.model_name);
    tracing::info!("PDP Threshold: {}", config.pdp_invalidation_threshold);
    tracing::info!("Max Batch Size: {}", config.max_batch_size);
    tracing::info!("Target Host: {}:{}", config.host, config.port);
    tracing::info!("===========================================================");

    let state = Arc::new(RuntimeState::new(config.clone()));
    let router = create_router(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Runtime listening on http://{}", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Comic OCR Runtime shutdown complete.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C signal, shutting down..."),
        _ = terminate => tracing::info!("Received terminate signal, shutting down..."),
    }
}
