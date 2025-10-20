use axum::{Router, routing::get, routing::post};
use std::sync::Arc;

use crate::{AppState, server::handlers};

/// Creates and configures the application router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/info", get(handlers::info_handler))
        .route("/api/v1/mint", post(handlers::mint_handler))
        .route("/api/v1/validate", post(handlers::validate_handler))
        .route(
            &format!("/ark:{}/servicestatus", state.naan),
            get(handlers::health_check_handler),
        )
        .route("/ark:{*ark_fragment}", get(handlers::resolve_handler))
        .with_state(state)
}
