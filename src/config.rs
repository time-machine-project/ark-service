use std::collections::HashMap;
use std::str::FromStr;

use crate::ark::is_betanumeric;
use crate::shoulder::{Shoulder, parse_shoulders};

/// The application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Lowercase betanumeric, as the NAAN appears in a normalized ARK.
    pub naan: String,
    /// Blade length without the check character, for shoulders without their own `blade_length`.
    pub default_blade_length: usize,
    /// Per request; larger requests are capped.
    pub max_mint_count: usize,
    pub shoulders: HashMap<String, Shoulder>,
}

impl AppState {
    /// Reads `NAAN`, `DEFAULT_BLADE_LENGTH`, `MAX_MINT_COUNT` and `SHOULDERS`.
    pub fn from_env() -> Result<Self, String> {
        let naan = std::env::var("NAAN")
            .map_err(|_| "NAAN is not set".to_string())?
            .to_ascii_lowercase();
        if naan.is_empty() || !is_betanumeric(&naan) {
            return Err(format!("NAAN {naan:?} is not betanumeric"));
        }
        let default_blade_length = env_or("DEFAULT_BLADE_LENGTH", 8)?;
        if default_blade_length == 0 {
            return Err("DEFAULT_BLADE_LENGTH must be at least 1".to_string());
        }
        let max_mint_count = env_or("MAX_MINT_COUNT", 1000)?;
        if max_mint_count == 0 {
            return Err("MAX_MINT_COUNT must be at least 1".to_string());
        }
        let shoulders =
            std::env::var("SHOULDERS").map_err(|_| "SHOULDERS is not set".to_string())?;

        Ok(Self {
            naan,
            default_blade_length,
            max_mint_count,
            shoulders: parse_shoulders(&shoulders)?,
        })
    }

    pub fn blade_length(&self, shoulder: &Shoulder) -> usize {
        shoulder.blade_length.unwrap_or(self.default_blade_length)
    }
}

/// The parsed value of an environment variable, or `default` when it is unset.
pub fn env_or<T: FromStr>(name: &str, default: T) -> Result<T, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("{name} has the invalid value {value:?}")),
        Err(_) => Ok(default),
    }
}
