use std::sync::Arc;

use tracing_subscriber::{EnvFilter, fmt};

use crate::config::{AppState, env_or};
use crate::server::router::create_router;

/// Runs the server with configuration loaded from environment variables
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let port: u16 = env_or("PORT", 3000)?;
    let state = AppState::from_env()?;

    let mut shoulders: Vec<&str> = state.shoulders.keys().map(String::as_str).collect();
    shoulders.sort_unstable();
    tracing::info!(
        naan = %state.naan,
        default_blade_length = state.default_blade_length,
        max_mint_count = state.max_mint_count,
        shoulders = ?shoulders,
        "Server configuration loaded"
    );

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);

    axum::serve(listener, create_router(Arc::new(state)))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Resolves on Ctrl-C or SIGTERM, which `docker stop` sends.
async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
