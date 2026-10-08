use axum::{Router, routing::get, routing::post};
use std::sync::Arc;

use crate::config::AppState;
use crate::server::handlers;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/info", get(handlers::info_handler))
        .route("/api/v1/mint", post(handlers::mint_handler))
        .route("/api/v1/validate", post(handlers::validate_handler))
        .route(
            &format!("/ark:{}/servicestatus", state.naan),
            get(handlers::health_check_handler),
        )
        // A catch-all, because the "ark:" label is case-insensitive
        .route("/{*path}", get(handlers::resolve_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shoulder::Shoulder;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode, header},
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn router() -> Router {
        let shoulder = Shoulder {
            route_pattern: "https://ark.timeatlas.eu/{pid}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        create_router(Arc::new(AppState {
            naan: "26341".to_string(),
            default_blade_length: 8,
            max_mint_count: 10,
            shoulders: HashMap::from([("b1".to_string(), shoulder)]),
        }))
    }

    async fn send(method: Method, uri: &str) -> axum::response::Response {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        router().oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn resolves_any_label_case() {
        for uri in [
            "/ark:26341/b1np1wh8k",
            "/ARK:26341/b1np1wh8k",
            "/Ark:/26341/b1np1wh8k",
        ] {
            let response = send(Method::GET, uri).await;
            assert_eq!(response.status(), StatusCode::FOUND, "{uri}");
            assert_eq!(
                response.headers()[header::LOCATION],
                "https://ark.timeatlas.eu/ark:26341/b1np1wh8k",
            );
        }
    }

    #[tokio::test]
    async fn serves_the_healthcheck() {
        let response = send(Method::GET, "/ark:26341/servicestatus").await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn answers_other_paths_with_not_found() {
        let response = send(Method::GET, "/favicon.ico").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rejects_dot_segments() {
        for uri in [
            "/ark:26341/b1np1wh8k/../../admin",
            "/ark:26341/b1np1wh8k/%2e%2E/admin",
            "/ark:26341/b1np1wh8k/./page2",
        ] {
            let response = send(Method::GET, uri).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
        }
    }

    #[tokio::test]
    async fn resolves_only_get_requests() {
        let response = send(Method::POST, "/ark:26341/b1np1wh8k").await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
