use crate::ark::BETANUMERIC;

/// Ordinal of each byte in the betanumeric alphabet; every other byte, uppercase letters
/// included, maps to 0 as in Noid.
const ORDINALS: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut ordinal = 0;
    while ordinal < BETANUMERIC.len() {
        table[BETANUMERIC[ordinal] as usize] = ordinal as u8;
        ordinal += 1;
    }
    table
};

/// Calculates the NCDA check character of a check zone without its check character.
///
/// For betanumeric strings of up to 28 characters, check character included, the
/// [Noid Check Digit Algorithm](https://metacpan.org/dist/Noid/view/noid#NOID-CHECK-DIGIT-ALGORITHM)
/// detects every single-character substitution and every transposition of two adjacent
/// characters.
///
/// ```
/// use ark_service::check_character::calculate_check_character;
///
/// // Example from the NCDA specification
/// assert_eq!(calculate_check_character("13030/xf93gt2"), 'q');
/// ```
pub fn calculate_check_character(check_zone: &str) -> char {
    let total: usize = check_zone
        .bytes()
        .enumerate()
        .map(|(i, b)| (i + 1) * usize::from(ORDINALS[b as usize]))
        .sum();
    BETANUMERIC[total % BETANUMERIC.len()] as char
}

/// Checks the last character of a complete check zone against the NCDA check character of
/// the rest; `false` for strings shorter than two characters.
///
/// ```
/// use ark_service::check_character::validate_check_character;
///
/// assert!(validate_check_character("13030/xf93gt2q"));
/// assert!(!validate_check_character("13030/xf93gt2x"));
/// assert!(!validate_check_character("a"));
/// ```
pub fn validate_check_character(check_zone: &str) -> bool {
    let Some(provided) = check_zone.chars().last() else {
        return false;
    };
    let base = &check_zone[..check_zone.len() - provided.len_utf8()];
    !base.is_empty() && provided == calculate_check_character(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn betanumeric_strings(length: usize) -> Vec<String> {
        (0..length).fold(vec![String::new()], |strings, _| {
            strings
                .iter()
                .flat_map(|s| {
                    BETANUMERIC
                        .iter()
                        .map(move |&c| format!("{s}{}", c as char))
                })
                .collect()
        })
    }

    fn with_check(base: &str) -> String {
        format!("{base}{}", calculate_check_character(base))
    }

    #[test]
    fn uppercase_letters_are_not_betanumeric() {
        assert!(!validate_check_character("13030/XF93GT2Q"));
        assert_ne!(
            calculate_check_character("13030/XF93GT2"),
            calculate_check_character("13030/xf93gt2")
        );
    }

    #[test]
    fn detects_every_substitution_in_short_strings() {
        for length in 1..=3 {
            for base in betanumeric_strings(length) {
                let correct = with_check(&base).into_bytes();
                for position in 0..correct.len() {
                    for &replacement in BETANUMERIC {
                        if replacement == correct[position] {
                            continue;
                        }
                        let mut wrong = correct.clone();
                        wrong[position] = replacement;
                        assert!(!validate_check_character(
                            std::str::from_utf8(&wrong).unwrap()
                        ));
                    }
                }
            }
        }
    }

    #[test]
    fn detects_every_adjacent_transposition_in_short_strings() {
        for length in 1..=3 {
            for base in betanumeric_strings(length) {
                let correct = with_check(&base).into_bytes();
                for position in 0..correct.len() - 1 {
                    if correct[position] == correct[position + 1] {
                        continue;
                    }
                    let mut swapped = correct.clone();
                    swapped.swap(position, position + 1);
                    assert!(!validate_check_character(
                        std::str::from_utf8(&swapped).unwrap()
                    ));
                }
            }
        }
    }
}
