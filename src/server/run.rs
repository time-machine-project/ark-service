use std::sync::Arc;

use crate::config::AppState;
use crate::server::router::create_router;
use crate::shoulder::load_shoulders_from_env;

/// Runs the server with configuration loaded from environment variables
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing to stdout
    use tracing_subscriber::{EnvFilter, fmt};

    // Set up env filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Configure formatter for Apache-like structured text logs
    fmt()
        .with_env_filter(env_filter)
        .with_target(false) // No Rust module paths
        .with_ansi(true) // Colors
        .with_level(true) // Show log level
        .with_thread_ids(false) // No thread IDs
        .with_thread_names(false) // No thread names
        .with_file(false) // No file names
        .with_line_number(false) // No line numbers
        .compact() // Compact format
        .init();

    // Load configuration from environment
    let naan = std::env::var("NAAN").unwrap_or_else(|_| {
        tracing::warn!("NAAN not set, using default: 12345");
        "12345".to_string()
    });

    let default_blade_length = std::env::var("DEFAULT_BLADE_LENGTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            tracing::warn!("DEFAULT_BLADE_LENGTH not set or invalid, using default: 8");
            8
        });

    let max_mint_count = std::env::var("MAX_MINT_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            tracing::warn!("MAX_MINT_COUNT not set or invalid, using default: 1000");
            1000
        });

    // Load shoulders from environment
    let shoulders = load_shoulders_from_env().unwrap_or_else(|e| {
        tracing::error!(
            error = %e,
            "Failed to load shoulder configuration from SHOULDERS environment variable"
        );
        std::process::exit(1);
    });

    tracing::info!(
        naan = %naan,
        default_blade_length = default_blade_length,
        max_mint_count = max_mint_count,
        shoulder_count = shoulders.len(),
        "Server configuration loaded"
    );

    for (shoulder, config) in &shoulders {
        tracing::debug!(
            shoulder = %shoulder,
            project_name = %config.project_name,
            route_pattern = %config.route_pattern,
            uses_check_character = config.uses_check_character,
            blade_length = ?config.blade_length,
            "Shoulder configuration"
        );
    }

    let state = Arc::new(AppState {
        naan,
        default_blade_length,
        max_mint_count,
        shoulders,
    });

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
