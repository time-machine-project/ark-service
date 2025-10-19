use rand::Rng;

use crate::check_character::calculate_check_character;
use crate::config::{AppState, BETANUMERIC};
use crate::error::AppError;

/// Mint a single new ARK with the given NAAN, shoulder, blade length, and check character option
pub fn mint_ark(
    naan: &str,
    shoulder: &str,
    blade_length: usize,
    uses_check_character: bool,
) -> String {
    let blade = generate_random_blade(blade_length);

    if uses_check_character {
        let identifier_for_check = format!("{}{}", shoulder, blade);
        let check_character = calculate_check_character(&identifier_for_check);
        format!("ark:{}/{}{}{}", naan, shoulder, blade, check_character)
    } else {
        format!("ark:{}/{}{}", naan, shoulder, blade)
    }
}

/// Mints multiple ARK identifiers for a given shoulder
///
/// # Arguments
/// * `state` - The application state containing NAAN and shoulder configurations
/// * `shoulder` - The shoulder identifier to mint ARKs for
/// * `count` - The number of ARKs to mint (will be capped at max_mint_count for safety)
///
/// # Returns
/// * `Ok(Vec<String>)` - Vector of minted ARK identifiers
/// * `Err(AppError)` - If the shoulder is not found
pub fn mint_arks(state: &AppState, shoulder: &str, count: usize) -> Result<Vec<String>, AppError> {
    // Verify shoulder exists and get its configuration
    let shoulder_config = state
        .shoulders
        .get(shoulder)
        .ok_or_else(|| {
            tracing::debug!(
                shoulder = %shoulder,
                "Mint failed: shoulder not found"
            );
            AppError::ShoulderNotFound
        })?;

    // Limit count for safety
    let original_count = count;
    let count = count.min(state.max_mint_count);

    if original_count > count {
        tracing::warn!(
            shoulder = %shoulder,
            requested_count = original_count,
            capped_count = count,
            max_mint_count = state.max_mint_count,
            "Mint request exceeded maximum, count capped"
        );
    }

    // Use shoulder-specific blade length if configured, otherwise use default
    let blade_length = shoulder_config
        .blade_length
        .unwrap_or(state.default_blade_length);

    tracing::debug!(
        shoulder = %shoulder,
        count = count,
        blade_length = blade_length,
        uses_check_character = shoulder_config.uses_check_character,
        "Minting ARKs"
    );

    // Generate ARKs with or without check characters based on shoulder config
    let arks: Vec<String> = (0..count)
        .map(|_| {
            mint_ark(
                &state.naan,
                shoulder,
                blade_length,
                shoulder_config.uses_check_character,
            )
        })
        .collect();

    Ok(arks)
}

/// Generate a random blade using betanumeric characters
fn generate_random_blade(blade_length: usize) -> String {
    let mut rng = rand::rng();
    (0..blade_length)
        .map(|_| {
            let idx = rng.random_range(0..BETANUMERIC.len());
            BETANUMERIC[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ark::parse_ark, config::BETANUMERIC, shoulder::Shoulder};
    use std::collections::HashMap;

    fn create_test_state(uses_check_character: bool) -> AppState {
        let mut shoulders = HashMap::new();
        shoulders.insert(
            "x6".to_string(),
            Shoulder {
                route_pattern: "https://example.org/${value}".to_string(),
                project_name: "Test Project".to_string(),
                uses_check_character,
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
    fn mints_requested_number_of_arks() {
        let state = create_test_state(true);
        let arks = mint_arks(&state, "x6", 5).unwrap();

        assert_eq!(arks.len(), 5);
        for ark in arks {
            assert!(ark.starts_with("ark:12345/x6"));
        }
    }

    #[test]
    fn enforces_maximum_count_limit() {
        let state = create_test_state(true);
        let arks = mint_arks(&state, "x6", 5000).unwrap();

        assert_eq!(arks.len(), 1000);
    }

    #[test]
    fn returns_error_for_invalid_shoulder() {
        let state = create_test_state(true);
        let result = mint_arks(&state, "invalid", 1);

        assert!(matches!(result, Err(AppError::ShoulderNotFound)));
    }

    #[test]
    fn mints_ark_with_check_character() {
        let ark = mint_ark("12345", "x6", 8, true);

        assert!(ark.starts_with("ark:12345/x6"));
        assert_eq!(ark.len(), "ark:12345/x6".len() + 9); // 8 blade + 1 check

        let parsed = parse_ark(&ark).unwrap();
        assert_eq!(parsed.naan, "12345");
        assert_eq!(parsed.shoulder, "x6");
        assert_eq!(parsed.blade.len(), 9);
    }

    #[test]
    fn mints_ark_without_check_character() {
        let ark = mint_ark("12345", "x6", 8, false);

        assert!(ark.starts_with("ark:12345/x6"));
        assert_eq!(ark.len(), "ark:12345/x6".len() + 8); // 8 blade only

        let parsed = parse_ark(&ark).unwrap();
        assert_eq!(parsed.naan, "12345");
        assert_eq!(parsed.shoulder, "x6");
        assert_eq!(parsed.blade.len(), 8);
    }

    #[test]
    fn generates_random_betanumeric_blades() {
        let blade1 = generate_random_blade(8);
        let blade2 = generate_random_blade(8);

        assert_eq!(blade1.len(), 8);
        assert_eq!(blade2.len(), 8);
        assert_ne!(blade1, blade2);

        for ch in blade1.chars().chain(blade2.chars()) {
            assert!(BETANUMERIC.contains(&(ch as u8)));
        }
    }

    #[test]
    fn uses_shoulder_specific_blade_length() {
        let mut shoulders = HashMap::new();
        // Shoulder with custom blade length
        shoulders.insert(
            "x6".to_string(),
            Shoulder {
                route_pattern: "https://example.org/${value}".to_string(),
                project_name: "Custom Length Project".to_string(),
                uses_check_character: false,
                blade_length: Some(12),
            },
        );
        // Shoulder using default blade length
        shoulders.insert(
            "b3".to_string(),
            Shoulder {
                route_pattern: "https://example.org/${value}".to_string(),
                project_name: "Default Length Project".to_string(),
                uses_check_character: false,
                ..Default::default()
            },
        );

        let state = AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders,
        };

        // Test shoulder with custom blade length (12 characters)
        let arks_x6 = mint_arks(&state, "x6", 1).unwrap();
        assert_eq!(arks_x6.len(), 1);
        let parsed_x6 = parse_ark(&arks_x6[0]).unwrap();
        assert_eq!(parsed_x6.blade.len(), 12); // Custom length

        // Test shoulder with default blade length (8 characters)
        let arks_b3 = mint_arks(&state, "b3", 1).unwrap();
        assert_eq!(arks_b3.len(), 1);
        let parsed_b3 = parse_ark(&arks_b3[0]).unwrap();
        assert_eq!(parsed_b3.blade.len(), 8); // Default length
    }

    #[test]
    fn uses_shoulder_blade_length_with_check_character() {
        let mut shoulders = HashMap::new();
        shoulders.insert(
            "fk4".to_string(),
            Shoulder {
                route_pattern: "https://example.org/${value}".to_string(),
                project_name: "Custom Length with Check".to_string(),
                blade_length: Some(10),
                ..Default::default()
            },
        );

        let state = AppState {
            naan: "99999".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders,
        };

        let arks = mint_arks(&state, "fk4", 1).unwrap();
        assert_eq!(arks.len(), 1);
        let parsed = parse_ark(&arks[0]).unwrap();
        // Blade should be 11 characters (10 + 1 check character)
        assert_eq!(parsed.blade.len(), 11);
        assert_eq!(parsed.naan, "99999");
        assert_eq!(parsed.shoulder, "fk4");
    }
}
