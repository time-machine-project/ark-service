use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum AppError {
    ShoulderNotFound,
    InvalidArk,
    InvalidNaan,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::ShoulderNotFound => {
                tracing::warn!(error_type = "ShoulderNotFound", "Request failed: shoulder not found");
                (StatusCode::NOT_FOUND, "Shoulder not found")
            }
            AppError::InvalidArk => {
                tracing::warn!(error_type = "InvalidArk", "Request failed: invalid ARK format");
                (StatusCode::BAD_REQUEST, "Invalid ARK format")
            }
            AppError::InvalidNaan => {
                tracing::warn!(error_type = "InvalidNaan", "Request failed: NAAN mismatch");
                (StatusCode::BAD_REQUEST, "NAAN does not match")
            }
        };

        (status, message).into_response()
    }
}

