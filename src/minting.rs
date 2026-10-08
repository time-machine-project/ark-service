use rand::RngExt;

use crate::ark::{BETANUMERIC, LABEL, check_zone};
use crate::check_character::calculate_check_character;
use crate::config::AppState;
use crate::error::AppError;

/// Mints one ARK; `blade_length` excludes the check character.
pub fn mint_ark(
    naan: &str,
    shoulder: &str,
    blade_length: usize,
    uses_check_character: bool,
) -> String {
    let mut zone = check_zone(naan, shoulder, &random_blade(blade_length));
    if uses_check_character {
        zone.push(calculate_check_character(&zone));
    }
    format!("{LABEL}{zone}")
}

/// Mints `count` ARKs for a registered shoulder, capped at `state.max_mint_count`.
pub fn mint_arks(state: &AppState, shoulder: &str, count: usize) -> Result<Vec<String>, AppError> {
    let config = state
        .shoulders
        .get(shoulder)
        .ok_or(AppError::ShoulderNotFound)?;

    let minted = count.min(state.max_mint_count);
    if minted < count {
        tracing::warn!(shoulder = ?shoulder, requested = count, minted, "Mint request capped");
    }

    let blade_length = state.blade_length(config);
    Ok((0..minted)
        .map(|_| {
            mint_ark(
                &state.naan,
                shoulder,
                blade_length,
                config.uses_check_character,
            )
        })
        .collect())
}

fn random_blade(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| BETANUMERIC[rng.random_range(0..BETANUMERIC.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark::parse_ark;
    use crate::check_character::validate_check_character;
    use crate::shoulder::Shoulder;
    use std::collections::HashMap;

    fn create_test_state(shoulders: &[(&str, Shoulder)]) -> AppState {
        AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders: shoulders
                .iter()
                .map(|(name, shoulder)| (name.to_string(), shoulder.clone()))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn with_check_character(uses_check_character: bool, blade_length: Option<usize>) -> Shoulder {
        Shoulder {
            uses_check_character,
            blade_length,
            ..Default::default()
        }
    }

    #[test]
    fn mints_requested_number_of_arks() {
        let state = create_test_state(&[("x6", Shoulder::default())]);
        let arks = mint_arks(&state, "x6", 5).unwrap();

        assert_eq!(arks.len(), 5);
        assert!(arks.iter().all(|ark| ark.starts_with("ark:12345/x6")));
    }

    #[test]
    fn caps_the_count_at_max_mint_count() {
        let state = create_test_state(&[("x6", Shoulder::default())]);
        assert_eq!(mint_arks(&state, "x6", 5000).unwrap().len(), 1000);
    }

    #[test]
    fn returns_error_for_unknown_shoulder() {
        let state = create_test_state(&[("x6", Shoulder::default())]);
        assert_eq!(mint_arks(&state, "z9", 1), Err(AppError::ShoulderNotFound));
    }

    #[test]
    fn appends_a_valid_check_character() {
        let ark = mint_ark("12345", "x6", 8, true);
        let parsed = parse_ark(&ark).unwrap();

        assert_eq!(parsed.shoulder(), Some("x6"));
        assert_eq!(parsed.blade().len(), 8 + 1);
        assert!(validate_check_character(&parsed.check_zone()));
    }

    #[test]
    fn mints_without_check_character() {
        let ark = mint_ark("12345", "x6", 8, false);
        assert_eq!(parse_ark(&ark).unwrap().blade().len(), 8);
    }

    #[test]
    fn generates_random_betanumeric_blades() {
        let blade1 = random_blade(8);
        let blade2 = random_blade(8);

        assert_eq!(blade1.len(), 8);
        assert_ne!(blade1, blade2);
        assert!(
            blade1
                .bytes()
                .chain(blade2.bytes())
                .all(|b| BETANUMERIC.contains(&b))
        );
    }

    #[test]
    fn uses_the_shoulder_blade_length_or_the_default() {
        let state = create_test_state(&[
            ("x6", with_check_character(false, Some(12))),
            ("b3", with_check_character(false, None)),
            ("fk4", with_check_character(true, Some(10))),
        ]);

        let blade = |shoulder| {
            parse_ark(&mint_arks(&state, shoulder, 1).unwrap()[0])
                .unwrap()
                .blade()
                .len()
        };
        assert_eq!(blade("x6"), 12);
        assert_eq!(blade("b3"), 8);
        assert_eq!(blade("fk4"), 10 + 1);
    }
}
