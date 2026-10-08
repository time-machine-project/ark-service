use std::fmt;

use serde::{Serialize, Serializer};

use crate::ark::{is_betanumeric, parse_ark};
use crate::check_character::validate_check_character;
use crate::config::AppState;

pub const MAX_ARKS_PER_REQUEST: usize = 1000;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub naan: Option<String>,
    pub shoulder: Option<String>,
    pub blade: Option<String>,
    pub naan_matches: Option<bool>,
    pub shoulder_registered: Option<bool>,
    pub has_check_character: Option<bool>,
    pub check_character_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    BladeNotBetanumeric,
    NaanMismatch,
    ShoulderNotRegistered,
    CheckCharacterMismatch,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Warning::BladeNotBetanumeric => "Blade contains non-betanumeric characters",
            Warning::NaanMismatch => "NAAN does not match this resolver",
            Warning::ShoulderNotRegistered => "Shoulder is not registered in this resolver",
            Warning::CheckCharacterMismatch => "Check character does not match",
        })
    }
}

impl Serialize for Warning {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Validates an ARK against draft-kunze-ark-43
///
/// `valid` reflects only the spec. Whether this resolver would redirect the ARK is reported in
/// `naan_matches` and `shoulder_registered`, and the check character is tested only when the
/// caller says the ARK has one.
pub fn validate_ark(
    state: &AppState,
    ark: &str,
    has_check_character: Option<bool>,
) -> ValidationResult {
    let parsed = match parse_ark(ark) {
        Ok(parsed) => parsed,
        Err(e) => {
            return ValidationResult {
                has_check_character,
                error: Some(e.to_string()),
                ..Default::default()
            };
        }
    };

    let mut warnings = Vec::new();
    if !is_betanumeric(parsed.blade()) {
        warnings.push(Warning::BladeNotBetanumeric);
    }

    let naan_matches = parsed.naan() == state.naan;
    let shoulder_registered = naan_matches.then(|| {
        parsed
            .shoulder()
            .is_some_and(|s| state.shoulders.contains_key(s))
    });
    if !naan_matches {
        warnings.push(Warning::NaanMismatch);
    } else if shoulder_registered == Some(false) {
        warnings.push(Warning::ShoulderNotRegistered);
    }

    let check_character_valid = (has_check_character == Some(true) && !parsed.blade().is_empty())
        .then(|| validate_check_character(&parsed.check_zone()));
    if check_character_valid == Some(false) {
        warnings.push(Warning::CheckCharacterMismatch);
    }

    let violation = parsed.violation();
    ValidationResult {
        valid: violation.is_none(),
        naan: Some(parsed.naan().to_string()),
        shoulder: parsed.shoulder().map(str::to_string),
        blade: Some(parsed.blade().to_string()),
        naan_matches: Some(naan_matches),
        shoulder_registered,
        has_check_character,
        check_character_valid,
        error: violation.map(|v| v.to_string()),
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shoulder::Shoulder;
    use std::collections::HashMap;

    fn create_test_state() -> AppState {
        AppState {
            naan: "12345".to_string(),
            default_blade_length: 8,
            max_mint_count: 1000,
            shoulders: HashMap::from([("x6".to_string(), Shoulder::default())]),
        }
    }

    #[test]
    fn reports_registered_ark_without_warnings() {
        let result = validate_ark(&create_test_state(), "ark:12345/x6np1wh8k", None);

        assert!(result.valid);
        assert_eq!(result.naan_matches, Some(true));
        assert_eq!(result.shoulder_registered, Some(true));
        assert_eq!(result.check_character_valid, None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn foreign_naan_is_valid_with_a_warning() {
        let result = validate_ark(&create_test_state(), "ark:99999/x6np1wh8k", None);

        assert!(result.valid);
        assert_eq!(result.naan_matches, Some(false));
        assert_eq!(result.shoulder_registered, None);
        assert_eq!(result.warnings, [Warning::NaanMismatch]);
    }

    #[test]
    fn unregistered_shoulder_is_valid_with_a_warning() {
        let result = validate_ark(&create_test_state(), "ark:12345/z9np1wh8k", None);

        assert!(result.valid);
        assert_eq!(result.naan_matches, Some(true));
        assert_eq!(result.shoulder_registered, Some(false));
        assert_eq!(result.warnings, [Warning::ShoulderNotRegistered]);
    }

    #[test]
    fn name_without_primordinal_shoulder_is_not_registered() {
        let result = validate_ark(&create_test_state(), "ark:12345/bcd", None);

        assert!(result.valid);
        assert_eq!(result.shoulder, None);
        assert_eq!(result.blade.as_deref(), Some("bcd"));
        assert_eq!(result.shoulder_registered, Some(false));
    }

    #[test]
    fn non_betanumeric_blade_is_valid_with_a_warning() {
        let result = validate_ark(
            &create_test_state(),
            "ark:12345/x6550e8400-e29b-41d4-a716-446655440000",
            None,
        );

        assert!(result.valid);
        assert_eq!(
            result.blade.as_deref(),
            Some("550e8400e29b41d4a716446655440000")
        );
        assert_eq!(result.warnings, [Warning::BladeNotBetanumeric]);
    }

    #[test]
    fn checks_the_check_character_only_on_request() {
        let state = create_test_state();
        let correct = crate::minting::mint_ark("12345", "x6", 8, true);
        let mut wrong = correct.clone();
        let last = wrong.pop().unwrap();
        wrong.push(if last == '0' { '1' } else { '0' });

        for has_check_character in [None, Some(false)] {
            let unchecked = validate_ark(&state, &wrong, has_check_character);
            assert_eq!(unchecked.has_check_character, has_check_character);
            assert_eq!(unchecked.check_character_valid, None);
        }

        let checked = validate_ark(&state, &correct, Some(true));
        assert_eq!(checked.check_character_valid, Some(true));

        let failed = validate_ark(&state, &wrong, Some(true));
        assert!(failed.valid);
        assert_eq!(failed.check_character_valid, Some(false));
        assert_eq!(failed.warnings, [Warning::CheckCharacterMismatch]);
    }

    #[test]
    fn checks_the_check_character_of_an_ark_with_qualifier_and_hyphens() {
        let ark = crate::minting::mint_ark("12345", "x6", 8, true);
        let (prefix, blade) = ark.split_at("ark:12345/x6".len());
        let hyphenated = format!("{prefix}{}-{}/page2.pdf", &blade[..4], &blade[4..]);

        let result = validate_ark(&create_test_state(), &hyphenated, Some(true));
        assert_eq!(result.check_character_valid, Some(true));
    }

    #[test]
    fn tests_no_check_character_without_a_blade() {
        let result = validate_ark(&create_test_state(), "ark:12345/x6", Some(true));

        assert_eq!(result.blade.as_deref(), Some(""));
        assert_eq!(result.check_character_valid, None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn reports_why_a_string_is_not_an_ark() {
        let result = validate_ark(&create_test_state(), "not-an-ark", None);

        assert!(!result.valid);
        assert_eq!(result.error.as_deref(), Some("Missing \"ark:\" label"));
        assert_eq!(result.naan_matches, None);
    }

    #[test]
    fn reports_spec_violations() {
        let result = validate_ark(&create_test_state(), "ark:12345/x6np,1wh8k", None);

        assert!(!result.valid);
        assert_eq!(
            result.error.as_deref(),
            Some("Name or qualifier contains characters outside the ARK repertoire")
        );
        assert_eq!(result.naan_matches, Some(true));
    }
}
