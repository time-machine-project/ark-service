use std::sync::LazyLock;

use crate::config::BETANUMERIC;

/// Pre-computed lookup table for O(1) betanumeric ordinal lookup.
/// Maps ASCII byte values (0-255) to their betanumeric ordinal (0-28).
/// Characters not in the betanumeric alphabet map to 0.
/// Both uppercase and lowercase letters map to the same ordinal.
///
/// Initialized lazily on first access using `LazyLock`.
static BETANUMERIC_LOOKUP: LazyLock<[u8; 256]> = LazyLock::new(|| {
    let mut table = [0u8; 256];

    // Map each betanumeric character to its ordinal (0-28)
    for (ordinal, &ch) in BETANUMERIC.iter().enumerate() {
        table[ch as usize] = ordinal as u8;

        // Also map uppercase version to same ordinal (for letters only)
        if ch.is_ascii_lowercase() {
            table[ch.to_ascii_uppercase() as usize] = ordinal as u8;
        }
    }

    table
});

/// Calculate the NCDA check character for a given identifier string.
///
/// This function implements the Noid Check Digit Algorithm (NCDA), which is a "perfect"
/// algorithm for detecting single character errors and transposition errors (swapping
/// adjacent characters) in identifiers.
///
/// The algorithm uses the "betanumeric" character set (digits 0-9 plus lowercase letters
/// excluding vowels and 'l'): `0123456789bcdfghjkmnpqrstvwxz` (29 characters, prime radix).
///
/// # Algorithm
///
/// For each character in the input string:
/// 1. Convert to lowercase
/// 2. Find its position in the betanumeric alphabet (0-28)
/// 3. Characters not in the alphabet get ordinal value 0 (e.g., '/')
/// 4. Multiply ordinal by position (1-indexed)
/// 5. Sum all products
/// 6. Check character is at position (sum mod 29) in the alphabet
///
/// See the [NOID Check Digit Algorithm specification](https://metacpan.org/dist/Noid/view/noid#NOID-CHECK-DIGIT-ALGORITHM)
/// for full details.
///
/// # Arguments
///
/// * `identifier` - The base identifier string (without check character)
///
/// # Returns
///
/// A single betanumeric character representing the check character
///
/// # Examples
///
/// ```
/// use ark_service::check_character::calculate_check_character;
///
/// // Example from NCDA specification
/// let check = calculate_check_character("13030/xf93gt2");
/// assert_eq!(check, 'q');
///
/// // Simple example
/// let check = calculate_check_character("bcd");
/// // 'b'=10, 'c'=11, 'd'=12 (ordinals in betanumeric)
/// // Position 1: 10 * 1 = 10
/// // Position 2: 11 * 2 = 22
/// // Position 3: 12 * 3 = 36
/// // Sum: 68, 68 mod 29 = 10 -> 'b'
/// assert_eq!(check, 'b');
/// ```
pub fn calculate_check_character(identifier: &str) -> char {
    let mut total: u64 = 0;

    for (position, ch) in identifier.bytes().enumerate() {
        // O(1) lookup instead of O(29) linear search
        let ordinal = BETANUMERIC_LOOKUP[ch as usize] as u64;

        total += (position as u64 + 1) * ordinal;
    }

    let check_ordinal = (total % 29) as usize;
    BETANUMERIC[check_ordinal] as char
}

/// Validate that an identifier has a correct check character.
///
/// This function extracts the last character from the identifier and verifies
/// it matches the expected check character calculated from the preceding characters.
///
/// # Arguments
///
/// * `identifier` - The complete identifier string (including check character)
///
/// # Returns
///
/// * `true` if the check character is valid
/// * `false` if the identifier is too short (< 2 chars) or check character is invalid
///
/// # Examples
///
/// ```
/// use ark_service::check_character::validate_check_character;
///
/// // Valid identifier with correct check character 'q'
/// assert!(validate_check_character("13030/xf93gt2q"));
///
/// // Invalid identifier with incorrect check character 'x'
/// assert!(!validate_check_character("13030/xf93gt2x"));
///
/// // Too short
/// assert!(!validate_check_character("a"));
/// ```
///
/// # Note
///
/// This function is case-insensitive since all characters are converted to
/// lowercase before processing.
pub fn validate_check_character(identifier: &str) -> bool {
    if identifier.len() < 2 {
        return false;
    }

    let (base, provided_check) = identifier.split_at(identifier.len() - 1);
    let expected_check = calculate_check_character(base);

    // Case-insensitive comparison
    provided_check.eq_ignore_ascii_case(&expected_check.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_character_calculation() {
        let identifier = "13030/xf93gt2";
        let check = calculate_check_character(identifier);
        assert_eq!(check, 'q');
    }

    #[test]
    fn test_check_character_validation() {
        assert!(validate_check_character("13030/xf93gt2q"));
        assert!(!validate_check_character("13030/xf93gt2x"));
    }

    #[test]
    fn test_case_insensitive() {
        // Verify that uppercase and lowercase identifiers produce the same check character
        assert_eq!(
            calculate_check_character("13030/XF93GT2"),
            calculate_check_character("13030/xf93gt2")
        );

        // Verify validation works with both cases
        assert!(validate_check_character("13030/XF93GT2Q"));
        assert!(validate_check_character("13030/xf93gt2q"));
        assert!(validate_check_character("13030/Xf93Gt2Q")); // Mixed case
    }
}
