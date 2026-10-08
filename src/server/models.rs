use serde::{Deserialize, Serialize};

use crate::validation::ValidationResult;

#[derive(Debug, Deserialize)]
pub struct MintRequest {
    pub shoulder: String,
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

#[derive(Debug, Deserialize)]
pub struct ValidateRequest {
    pub arks: Vec<String>,
    pub has_check_character: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MintResponse {
    pub arks: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub results: Vec<ArkValidationResult>,
}

#[derive(Debug, Serialize)]
pub struct ArkValidationResult {
    pub ark: String,
    #[serde(flatten)]
    pub result: ValidationResult,
}

#[derive(Debug, Serialize)]
pub struct ShoulderInfo {
    pub shoulder: String,
    pub project_name: String,
    pub uses_check_character: bool,
    pub blade_length: usize,
    pub example_ark: String,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub naan: String,
    pub shoulders: Vec<ShoulderInfo>,
}
