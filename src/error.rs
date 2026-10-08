use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::validation::MAX_ARKS_PER_REQUEST;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppError {
    ShoulderNotFound,
    InvalidArk,
    InvalidNaan,
    InvalidTarget,
    TooManyArks,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::warn!(error = ?self, "Request failed");
        let (status, message) = match self {
            AppError::ShoulderNotFound => (StatusCode::NOT_FOUND, "Shoulder not found".to_string()),
            AppError::InvalidArk => (StatusCode::BAD_REQUEST, "Invalid ARK format".to_string()),
            AppError::InvalidNaan => (StatusCode::BAD_REQUEST, "NAAN does not match".to_string()),
            AppError::InvalidTarget => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid redirect target".to_string(),
            ),
            AppError::TooManyArks => (
                StatusCode::BAD_REQUEST,
                format!("Too many ARKs: at most {MAX_ARKS_PER_REQUEST} per request"),
            ),
        };
        (status, message).into_response()
    }
}
