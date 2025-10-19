use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MintRequest {
    pub shoulder: String,
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub arks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub valid: bool,
    pub naan: Option<String>,
    pub shoulder: Option<String>,
    pub blade: Option<String>,
    pub shoulder_registered: Option<bool>,
    pub has_check_character: Option<bool>,
    pub check_character_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ShoulderInfo {
    pub shoulder: String,
    pub project_name: String,
    pub uses_check_character: bool,
    pub example_ark: String,
}

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub naan: String,
    pub shoulders: Vec<ShoulderInfo>,
}
