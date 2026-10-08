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
use crate::ark::has_label;
use crate::config::AppState;
use crate::error::AppError;
use crate::minting::{self, mint_ark};
use crate::resolution;
use crate::validation::{self, MAX_ARKS_PER_REQUEST};

pub async fn health_check_handler() -> &'static str {
    "OK"
}

pub async fn info_handler(State(state): State<Arc<AppState>>) -> Json<InfoResponse> {
    let mut shoulders: Vec<ShoulderInfo> = state
        .shoulders
        .iter()
        .map(|(name, config)| {
            let blade_length = state.blade_length(config);
            ShoulderInfo {
                shoulder: name.clone(),
                project_name: config.project_name.clone(),
                uses_check_character: config.uses_check_character,
                blade_length,
                example_ark: mint_ark(&state.naan, name, blade_length, config.uses_check_character),
            }
        })
        .collect();
    shoulders.sort_by(|a, b| a.shoulder.cmp(&b.shoulder));

    Json(InfoResponse {
        naan: state.naan.clone(),
        shoulders,
    })
}

pub async fn mint_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MintRequest>,
) -> Result<Json<MintResponse>, AppError> {
    let arks = minting::mint_arks(&state, &payload.shoulder, payload.count)?;
    tracing::info!(shoulder = ?payload.shoulder, count = arks.len(), "ARKs minted");

    Ok(Json(MintResponse {
        count: arks.len(),
        arks,
    }))
}

pub async fn validate_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, AppError> {
    if payload.arks.len() > MAX_ARKS_PER_REQUEST {
        return Err(AppError::TooManyArks);
    }

    let results: Vec<ArkValidationResult> = payload
        .arks
        .into_iter()
        .map(|ark| ArkValidationResult {
            result: validation::validate_ark(&state, &ark, payload.has_check_character),
            ark,
        })
        .collect();

    let valid = results.iter().filter(|r| r.result.valid).count();
    tracing::info!(total = results.len(), valid, "ARKs validated");

    Ok(Json(ValidateResponse { results }))
}

pub async fn resolve_handler(
    State(state): State<Arc<AppState>>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, AppError> {
    let path_and_query = uri.path_and_query().map_or("", |p| p.as_str());
    let Some(received) = path_and_query.strip_prefix('/').filter(|p| has_label(p)) else {
        return Ok((StatusCode::NOT_FOUND, "Not found").into_response());
    };

    let target = resolution::resolve(&state, received)?;
    Ok((StatusCode::FOUND, [(header::LOCATION, target)]).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark::parse_ark;
    use crate::check_character::validate_check_character;
    use crate::shoulder::Shoulder;
    use std::collections::HashMap;

    fn create_test_state() -> Arc<AppState> {
        let shoulder = |uses_check_character| Shoulder {
            route_pattern: "https://example.org/${value}".to_string(),
            project_name: "Test Project".to_string(),
            uses_check_character,
            ..Default::default()
        };
        Arc::new(AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders: HashMap::from([
                ("x6".to_string(), shoulder(true)),
                ("b3".to_string(), shoulder(false)),
            ]),
        })
    }

    #[tokio::test]
    async fn info_lists_shoulders_in_order_with_valid_example_arks() {
        let Json(info) = info_handler(State(create_test_state())).await;

        assert_eq!(info.naan, "12345");
        let names: Vec<&str> = info.shoulders.iter().map(|s| s.shoulder.as_str()).collect();
        assert_eq!(names, ["b3", "x6"]);

        let x6 = &info.shoulders[1];
        assert_eq!(x6.blade_length, 8);
        let example = parse_ark(&x6.example_ark).unwrap();
        assert_eq!(example.shoulder(), Some("x6"));
        assert!(validate_check_character(&example.check_zone()));
    }

    #[tokio::test]
    async fn mints_for_a_registered_shoulder() {
        let payload = MintRequest {
            shoulder: "x6".to_string(),
            count: 3,
        };
        let Json(response) = mint_handler(State(create_test_state()), Json(payload))
            .await
            .unwrap();

        assert_eq!(response.count, 3);
        assert!(
            response
                .arks
                .iter()
                .all(|ark| ark.starts_with("ark:12345/x6"))
        );
    }

    #[tokio::test]
    async fn rejects_minting_for_an_unknown_shoulder() {
        let payload = MintRequest {
            shoulder: "z9".to_string(),
            count: 1,
        };
        let result = mint_handler(State(create_test_state()), Json(payload)).await;
        assert_eq!(result.err(), Some(AppError::ShoulderNotFound));
    }

    #[tokio::test]
    async fn validates_each_ark_in_order() {
        let payload = ValidateRequest {
            arks: vec!["ark:12345/x6np1wh8k".to_string(), "doi:10.1/x".to_string()],
            has_check_character: None,
        };
        let Json(response) = validate_handler(State(create_test_state()), Json(payload))
            .await
            .unwrap();

        assert_eq!(response.results[0].ark, "ark:12345/x6np1wh8k");
        assert!(response.results[0].result.valid);
        assert_eq!(response.results[0].result.shoulder_registered, Some(true));
        assert_eq!(response.results[1].ark, "doi:10.1/x");
        assert!(!response.results[1].result.valid);
    }

    #[tokio::test]
    async fn rejects_more_arks_than_the_limit() {
        let payload = ValidateRequest {
            arks: vec![String::new(); MAX_ARKS_PER_REQUEST + 1],
            has_check_character: None,
        };
        let result = validate_handler(State(create_test_state()), Json(payload)).await;
        assert_eq!(result.err(), Some(AppError::TooManyArks));
    }
}
