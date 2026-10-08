use std::sync::Arc;

use tracing_subscriber::{EnvFilter, fmt};

use crate::config::{AppState, env_or};
use crate::server::router::create_router;

/// Runs the server with configuration loaded from environment variables
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Logging starts with `info` on an invalid RUST_LOG, so the startup error gets logged.
    let (env_filter, filter_error) = match log_filter(std::env::var("RUST_LOG").ok().as_deref()) {
        Ok(filter) => (filter, None),
        Err(e) => (EnvFilter::new("info"), Some(e)),
    };
    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
    if let Some(e) = filter_error {
        return Err(e.into());
    }

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

/// The filter from the value of `RUST_LOG`, or `info` when it is unset.
fn log_filter(directives: Option<&str>) -> Result<EnvFilter, String> {
    match directives {
        Some(directives) => EnvFilter::try_new(directives)
            .map_err(|e| format!("RUST_LOG has the invalid value {directives:?}: {e}")),
        None => Ok(EnvFilter::new("info")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_levels_and_per_crate_directives() {
        assert!(log_filter(None).is_ok());
        assert!(log_filter(Some("warn")).is_ok());
        assert!(log_filter(Some("warn,ark_service=debug")).is_ok());
    }

    #[test]
    fn rejects_an_invalid_filter() {
        let error = log_filter(Some("ark_service=loud")).unwrap_err();
        assert!(error.starts_with("RUST_LOG has the invalid value \"ark_service=loud\""));
    }
}
