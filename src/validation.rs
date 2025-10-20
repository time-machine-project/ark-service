use crate::ark::parse_ark;
use crate::check_character::validate_check_character;
use crate::config::{AppState, BETANUMERIC};

/// Result of ARK validation
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    pub naan: Option<String>,
    pub shoulder: Option<String>,
    pub blade: Option<String>,
    pub shoulder_registered: Option<bool>,
    pub has_check_character: Option<bool>,
    pub check_character_valid: Option<bool>,
    pub error: Option<String>,
    pub warnings: Option<Vec<String>>,
}

impl ValidationResult {
    /// Creates a validation result for a parsing error
    pub fn parse_error() -> Self {
        Self {
            valid: false,
            naan: None,
            shoulder: None,
            blade: None,
            shoulder_registered: None,
            has_check_character: None,
            check_character_valid: None,
            error: Some("Failed to parse ARK structure".to_string()),
            warnings: None,
        }
    }
}

/// Validates an ARK identifier
pub fn validate_ark(
    state: &AppState,
    ark: &str,
    has_check_character: Option<bool>,
) -> ValidationResult {
    // Parse ARK
    let Some(parsed) = parse_ark(ark) else {
        tracing::debug!(
            ark = %ark,
            "Validation failed: invalid ARK format"
        );
        return ValidationResult::parse_error();
    };

    // Validate betanumeric characters in shoulder and blade
    if !is_betanumeric(&parsed.shoulder) || !is_betanumeric(&parsed.blade) {
        tracing::debug!(
            ark = %ark,
            shoulder = %parsed.shoulder,
            blade = %parsed.blade,
            "Validation failed: non-betanumeric characters"
        );
        return ValidationResult {
            valid: false,
            naan: Some(parsed.naan),
            shoulder: Some(parsed.shoulder),
            blade: Some(parsed.blade),
            shoulder_registered: None,
            has_check_character: None,
            check_character_valid: None,
            error: Some(
                "Shoulder and blade must contain only betanumeric characters (0-9, b-z excluding vowels)".to_string()
            ),
            warnings: None,
        };
    }

    // Check if NAAN matches
    let naan_matches = parsed.naan == state.naan;
    let naan_error = if !naan_matches {
        Some(format!(
            "NAAN {} does not match configured NAAN {}",
            parsed.naan, state.naan
        ))
    } else {
        None
    };

    // Check if shoulder is registered
    let shoulder_config = state.shoulders.get(&parsed.shoulder);
    let shoulder_registered = shoulder_config.is_some();

    // Determine if check character should be validated
    let should_validate_check = match has_check_character {
        Some(has_check) => Some(has_check),
        None => {
            // Check shoulder configuration
            shoulder_config.map(|c| c.uses_check_character)
        }
    };

    // Strict mode: if shoulder is not registered and no hint provided, return error
    let Some(should_validate_check) = should_validate_check else {
        tracing::debug!(
            ark = %ark,
            shoulder = %parsed.shoulder,
            "Validation failed: unknown shoulder and no check character hint provided"
        );
        return ValidationResult {
            valid: false,
            naan: Some(parsed.naan),
            shoulder: Some(parsed.shoulder),
            blade: Some(parsed.blade),
            shoulder_registered: Some(false),
            has_check_character: None,
            check_character_valid: None,
            error: Some(
                "Unknown shoulder. Please specify has_check_character parameter to validate unregistered shoulders.".to_string()
            ),
            warnings: None,
        };
    };

    // Check character validation requires blade length > 1 because:
    // - At least 1 character is needed for the base identifier
    // - The last character is the check character to validate
    // Example: blade "ab" -> base "a" + check char "b"
    let (check_character_valid, warnings) = if should_validate_check && parsed.blade.len() > 1 {
        let identifier_for_check = format!("{}{}", parsed.shoulder, parsed.blade);
        let is_valid = validate_check_character(&identifier_for_check);

        let mut warnings_list = Vec::new();
        if !is_valid {
            warnings_list.push(
                "Check character validation failed. Either there's an error or this ARK has no check character."
                    .to_string(),
            );
        }
        if !shoulder_registered {
            warnings_list.push("Shoulder is not registered in the system.".to_string());
        }

        (
            Some(is_valid),
            if warnings_list.is_empty() {
                None
            } else {
                Some(warnings_list)
            },
        )
    } else if !should_validate_check {
        (Some(true), None)
    } else {
        (
            None,
            Some(vec![
                "Blade too short for check character validation".to_string(),
            ]),
        )
    };

    let valid = naan_matches && check_character_valid.unwrap_or(true) && shoulder_registered;

    ValidationResult {
        valid,
        naan: Some(parsed.naan),
        shoulder: Some(parsed.shoulder),
        blade: Some(parsed.blade),
        shoulder_registered: Some(shoulder_registered),
        has_check_character: Some(should_validate_check),
        check_character_valid,
        error: naan_error,
        warnings,
    }
}

/// Checks if a string contains only valid betanumeric characters
fn is_betanumeric(s: &str) -> bool {
    s.bytes().all(|b| BETANUMERIC.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shoulder::Shoulder;
    use std::collections::HashMap;

    fn create_test_state() -> AppState {
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

        AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders,
        }
    }

    #[test]
    fn test_validate_valid_ark_with_check_char() {
        let state = create_test_state();
        // Valid ARK with check character: ark:/12345/x6np1wh8f
        let result = validate_ark(&state, "ark:/12345/x6np1wh8f", Some(true));

        assert!(result.valid);
        assert_eq!(result.naan, Some("12345".to_string()));
        assert_eq!(result.shoulder, Some("x6".to_string()));
        assert_eq!(result.blade, Some("np1wh8f".to_string()));
        assert_eq!(result.shoulder_registered, Some(true));
        assert_eq!(result.check_character_valid, Some(true));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_validate_invalid_check_char() {
        let state = create_test_state();
        // Invalid check character
        let result = validate_ark(&state, "ark:/12345/x6np1wh8x", Some(true)); // Wrong check char

        assert!(!result.valid);
        assert_eq!(result.check_character_valid, Some(false));
        assert!(result.warnings.is_some());
    }

    #[test]
    fn test_validate_wrong_naan() {
        let state = create_test_state();
        let result = validate_ark(&state, "ark:/99999/x6nmkd123", None);

        assert!(!result.valid);
        assert_eq!(result.naan, Some("99999".to_string()));
        assert_eq!(result.shoulder, Some("x6".to_string()));
        assert_eq!(result.blade, Some("nmkd123".to_string()));
        assert_eq!(result.shoulder_registered, Some(true)); // x6 is registered
        assert!(result.has_check_character.is_some());
        assert!(result.check_character_valid.is_some());
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("does not match"));
    }

    #[test]
    fn test_validate_unregistered_shoulder() {
        let state = create_test_state();
        let result = validate_ark(&state, "ark:/12345/z9nmkd123", Some(false));

        assert!(!result.valid);
        assert_eq!(result.shoulder_registered, Some(false));
    }

    #[test]
    fn test_validate_invalid_ark_format() {
        let state = create_test_state();
        let result = validate_ark(&state, "not-an-ark", None);

        assert!(!result.valid);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Failed to parse ARK structure");
    }

    #[test]
    fn test_validate_no_check_char_shoulder() {
        let state = create_test_state();
        // b3 shoulder doesn't use check characters
        let result = validate_ark(&state, "ark:/12345/b3nmkd123", None); // Will use shoulder config

        assert!(result.valid);
        assert_eq!(result.has_check_character, Some(false));
        assert_eq!(result.check_character_valid, Some(true)); // Skipped validation
    }

    #[test]
    fn test_validate_blade_too_short() {
        let state = create_test_state();
        let result = validate_ark(&state, "ark:/12345/x6b", Some(true)); // Blade is only "b"

        // Blade too short for check character validation, but shoulder is registered
        // so it should be valid with a warning
        assert!(result.valid);
        assert_eq!(result.shoulder_registered, Some(true));
        assert_eq!(result.check_character_valid, None); // No validation performed
        assert!(result.warnings.is_some());
        let warnings = result.warnings.unwrap();
        assert!(warnings.iter().any(|w| w.contains("too short")));
    }

    #[test]
    fn test_validate_invalid_shoulder_characters() {
        let state = create_test_state();
        // Shoulder with vowel 'a'
        let result = validate_ark(&state, "ark:/12345/a6nmkd123", None);

        assert!(!result.valid);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("betanumeric"));
    }

    #[test]
    fn test_validate_invalid_blade_characters() {
        let state = create_test_state();
        // Blade with uppercase letter
        let result = validate_ark(&state, "ark:/12345/x6Nmkd123", None);

        assert!(!result.valid);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("betanumeric"));
    }

    #[test]
    fn test_validate_invalid_blade_with_vowel() {
        let state = create_test_state();
        // Blade with vowel 'e'
        let result = validate_ark(&state, "ark:/12345/x6nmked123", None);

        assert!(!result.valid);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("betanumeric"));
    }

    #[test]
    fn test_validate_invalid_blade_with_special_char() {
        let state = create_test_state();
        // Blade with special character '@'
        let result = validate_ark(&state, "ark:/12345/x6nmkd@123", None);

        assert!(!result.valid);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("betanumeric"));
    }
}
