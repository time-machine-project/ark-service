use axum::{
    Json,
    extract::{OriginalUri, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::models::{
    ArkValidationResult, InfoResponse, MintRequest, MintResponse, ShoulderInfo, ValidateRequest,
    ValidateResponse,
};
use crate::config::AppState;
use crate::error::AppError;
use crate::minting;
use crate::validation;
use crate::{ark::Ark, minting::mint_ark};

pub async fn health_check_handler() -> &'static str {
    "OK"
}

pub async fn info_handler(State(state): State<Arc<AppState>>) -> Json<InfoResponse> {
    let shoulders: Vec<ShoulderInfo> = state
        .shoulders
        .iter()
        .map(|(shoulder, config)| {
            let blade_length = config.blade_length.unwrap_or(state.default_blade_length);
            ShoulderInfo {
                shoulder: shoulder.clone(),
                project_name: config.project_name.clone(),
                uses_check_character: config.uses_check_character,
                blade_length,
                example_ark: mint_ark(
                    &state.naan,
                    shoulder,
                    blade_length,
                    config.uses_check_character,
                ),
            }
        })
        .collect();

    tracing::debug!(shoulder_count = shoulders.len(), "Info request");

    Json(InfoResponse {
        naan: state.naan.clone(),
        shoulders,
    })
}

pub async fn mint_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MintRequest>,
) -> Result<Json<MintResponse>, AppError> {
    tracing::info!(
        shoulder = %payload.shoulder,
        requested_count = payload.count,
        "Mint request received"
    );

    let arks = minting::mint_arks(&state, &payload.shoulder, payload.count)?;

    tracing::info!(
        shoulder = %payload.shoulder,
        minted_count = arks.len(),
        requested_count = payload.count,
        "Mint request completed successfully"
    );

    Ok(Json(MintResponse {
        count: arks.len(),
        arks,
    }))
}

pub async fn validate_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ValidateRequest>,
) -> Json<ValidateResponse> {
    let results: Vec<ArkValidationResult> = payload
        .arks
        .iter()
        .map(|ark| {
            let result = validation::validate_ark(&state, ark, payload.has_check_character);

            ArkValidationResult {
                ark: ark.clone(),
                valid: result.valid,
                naan: result.naan,
                shoulder: result.shoulder,
                blade: result.blade,
                shoulder_registered: result.shoulder_registered,
                has_check_character: result.has_check_character,
                check_character_valid: result.check_character_valid,
                error: result.error,
                warnings: result.warnings,
            }
        })
        .collect();

    let valid_count = results.iter().filter(|r| r.valid).count();
    let invalid_count = results.len() - valid_count;

    if invalid_count > 0 {
        tracing::info!(
            total = results.len(),
            valid = valid_count,
            invalid = invalid_count,
            "Validation completed with failures"
        );
    } else {
        tracing::debug!(total = results.len(), "Validation completed - all valid");
    }

    Json(ValidateResponse { results })
}

pub async fn resolve_handler(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, AppError> {
    // Extract path and query from URI: /ark:12345/x6test?info -> ark:12345/x6test?info
    let path_and_query = uri.path_and_query().ok_or(AppError::InvalidArk)?.as_str();

    // Remove leading /ark: to get just the ARK identifier
    let ark_string = path_and_query
        .strip_prefix("/ark:")
        .ok_or(AppError::InvalidArk)?;

    let ark_string = format!("ark:{}", ark_string);
    // Parse the full ARK string (e.g., "ark:12345/x6np1wh8k/page2.pdf?info")
    let parsed_ark = Ark::try_from(ark_string.as_str())?;

    // Check NAAN matches
    if parsed_ark.naan != state.naan {
        return Err(AppError::InvalidNaan);
    }

    // Look up routing rule
    let shoulder_config = state
        .shoulders
        .get(&parsed_ark.shoulder)
        .ok_or(AppError::ShoulderNotFound)?;

    // Resolve ARK using shoulder's routing configuration
    let target_url = shoulder_config.resolve(&parsed_ark);

    tracing::debug!(
        shoulder = %parsed_ark.shoulder,
        "ARK resolved"
    );

    // Create a 302 Found redirect
    Ok((StatusCode::FOUND, [(header::LOCATION, target_url)]).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shoulder::Shoulder;
    use std::collections::HashMap;

    fn create_test_state() -> Arc<AppState> {
        let mut shoulders = HashMap::new();
        shoulders.insert(
            "x6".to_string(),
            Shoulder {
                route_pattern: "https://example.org/${value}".to_string(),
                project_name: "Test Project".to_string(),
                ..Default::default()
            },
        );
        shoulders.insert(
            "b3".to_string(),
            Shoulder {
                route_pattern: "https://beta.org/items/${value}".to_string(),
                project_name: "Beta Project".to_string(),
                uses_check_character: false,
                ..Default::default()
            },
        );

        Arc::new(AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders,
        })
    }

    #[tokio::test]
    async fn test_health_check_handler() {
        let result = health_check_handler().await;
        assert_eq!(result, "OK");
    }

    #[tokio::test]
    async fn test_info_handler_returns_shoulder_info() {
        let state = create_test_state();
        let response = info_handler(State(state.clone())).await;

        assert_eq!(response.0.naan, "12345");
        assert_eq!(response.0.shoulders.len(), 2);

        // Check that shoulders are present
        let shoulder_names: Vec<&str> = response
            .0
            .shoulders
            .iter()
            .map(|s| s.shoulder.as_str())
            .collect();
        assert!(shoulder_names.contains(&"x6"));
        assert!(shoulder_names.contains(&"b3"));
    }

    #[tokio::test]
    async fn test_mint_handler_success() {
        let state = create_test_state();
        let payload = MintRequest {
            shoulder: "x6".to_string(),
            count: 3,
        };

        let result = mint_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.0.count, 3);
        assert_eq!(response.0.arks.len(), 3);

        // Verify ARKs have correct format (starts with ark:12345/x6)
        for ark in &response.0.arks {
            assert!(ark.starts_with("ark:12345/x6"));
        }
    }

    #[tokio::test]
    async fn test_mint_handler_invalid_shoulder() {
        let state = create_test_state();
        let payload = MintRequest {
            shoulder: "z9".to_string(), // Unregistered shoulder
            count: 1,
        };

        let result = mint_handler(State(state), Json(payload)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ShoulderNotFound));
    }

    #[tokio::test]
    async fn test_validate_handler_returns_results() {
        let state = create_test_state();
        let payload = ValidateRequest {
            arks: vec![
                "ark:12345/x6test123".to_string(),
                "ark:12345/b3data456".to_string(),
            ],
            has_check_character: None,
        };

        let response = validate_handler(State(state), Json(payload)).await;
        assert_eq!(response.0.results.len(), 2);

        // Verify handler returns results for each ARK
        assert_eq!(response.0.results[0].ark, "ark:12345/x6test123");
        assert_eq!(response.0.results[1].ark, "ark:12345/b3data456");
    }

    #[tokio::test]
    async fn test_resolve_handler_success() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:12345/x6np1wh8k");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_ok());

        // Handler returns a redirect - verify it produces a response
        let response = result.unwrap().into_response();
        assert_eq!(response.status(), StatusCode::FOUND);

        // Verify Location header is set
        let location = response.headers().get(header::LOCATION).unwrap();
        assert_eq!(location, "https://example.org/x6np1wh8k");
    }

    #[tokio::test]
    async fn test_resolve_handler_with_qualifier() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:12345/x6np1wh8k/page2.pdf");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_ok());

        // Handler returns a redirect - verify it produces a response
        let response = result.unwrap().into_response();
        assert_eq!(response.status(), StatusCode::FOUND);

        // Verify Location header is set with qualifier
        let location = response.headers().get(header::LOCATION).unwrap();
        assert_eq!(location, "https://example.org/x6np1wh8k/page2.pdf");
    }

    #[tokio::test]
    async fn test_resolve_handler_invalid_naan() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:99999/x6np1wh8k");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::InvalidNaan));
    }

    #[tokio::test]
    async fn test_resolve_handler_shoulder_not_found() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:12345/z9unknown");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ShoulderNotFound));
    }

    #[tokio::test]
    async fn test_resolve_handler_invalid_ark_format() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:invalid");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::InvalidArk));
    }

    #[tokio::test]
    async fn test_resolve_handler_with_query_string() {
        let state = create_test_state();
        let uri = axum::http::Uri::from_static("/ark:12345/x6np1wh8k?info");

        let result = resolve_handler(State(state), OriginalUri(uri)).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_response();
        assert_eq!(response.status(), StatusCode::FOUND);

        // Verify Location header includes query string
        let location = response.headers().get(header::LOCATION).unwrap();
        assert_eq!(location, "https://example.org/x6np1wh8k?info");
    }
}
