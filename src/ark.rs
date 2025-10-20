use crate::AppError;

/// An ARK identifier parsed into its components
///
/// This struct stores components in their original form (preserving hyphens, case, query strings, etc.)
/// for use in resolution and forwarding. The `normalized_ark` field contains a fully
/// normalized version used only for equality comparison per RFC specifications.
#[derive(Debug, Clone)]
pub struct Ark {
    /// The original ARK string as received (only ark:/ normalized to ark)
    pub original: String,
    /// The NAAN (Name Assigning Authority Number) as received
    pub naan: String,
    /// The shoulder (prefix) of the ARK as received
    pub shoulder: String,
    /// The blade (unique identifier) of the ARK as received
    pub blade: String,
    /// The qualifier (optional additional path) of the ARK as received. This includes any query
    /// string.
    pub qualifier: String,
    /// Fully normalized ARK for equality comparison only (lowercase NAAN, hyphens removed, etc.)
    pub normalized_ark: String,
}

impl PartialEq for Ark {
    fn eq(&self, other: &Self) -> bool {
        // Equality is based solely on the normalized form per RFC
        self.normalized_ark == other.normalized_ark
    }
}

impl Eq for Ark {}

impl TryFrom<&str> for Ark {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_ark(value).ok_or(AppError::InvalidArk)
    }
}

impl TryFrom<String> for Ark {
    type Error = AppError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

/// Extract shoulder from ARK path (primordial shoulder: letters ending with first digit)
pub fn extract_shoulder(path: &str) -> Option<&str> {
    for (byte_idx, ch) in path.char_indices() {
        if ch.is_ascii_digit() {
            return Some(&path[..=byte_idx]);
        }
    }
    None
}

/// Normalize an ARK string according to RFC specifications
/// Returns a fully normalized ARK suitable for comparison
fn normalize_ark_string(ark: &str) -> String {
    // Remove query string (everything from first '?' onwards)
    let ark = ark.split('?').next().unwrap_or(ark);

    // Handle both ark: and ark:/ formats
    let ark = ark.replace("ark:/", "ark:");

    // Remove whitespace (spaces, tabs, newlines, etc.) that may have been introduced
    // during text wrapping or copy-paste operations
    let mut ark = ark
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    // Remove hyphens (standard ASCII hyphen)
    ark = ark.replace("-", "");

    // Remove hyphen-like characters
    ark = ark.replace('\u{2010}', ""); // U+2010: ‐ (HYPHEN)
    ark = ark.replace('\u{2011}', ""); // U+2011: ‑ (NON-BREAKING HYPHEN)
    ark = ark.replace('\u{2012}', ""); // U+2012: ‒ (FIGURE DASH)
    ark = ark.replace('\u{2013}', ""); // U+2013: – (EN DASH)
    ark = ark.replace('\u{2014}', ""); // U+2014: — (EM DASH)
    ark = ark.replace('\u{2015}', ""); // U+2015: ― (HORIZONTAL BAR)

    // Lowercase the NAAN
    if let Some(slash_pos) = ark.find('/').filter(|&pos| pos > 4) {
        // Split into "ark:NAAN" and rest
        let (prefix, rest) = ark.split_at(slash_pos);
        let naan_part = &prefix[4..]; // Skip "ark:"
        ark = format!("ark:{}{}", naan_part.to_lowercase(), rest);
    }

    // Strip trailing structural characters (/ and .) from the end
    ark = ark.trim_end_matches(&['/', '.'][..]).to_string();

    ark
}

/// Parse an ARK identifier into its components
///
/// Parses an ARK and stores components in their original form (preserving hyphens, case, query strings, etc.)
/// except for ark:/ -> ark: conversion. A fully normalized version is computed and stored internally
/// for equality comparison (which removes query strings per RFC).
pub fn parse_ark(ark: &str) -> Option<Ark> {
    // Minimal normalization - ONLY normalize ark:/ to ark:
    let original_form = ark.replace("ark:/", "ark:");

    if !original_form.starts_with("ark:") {
        return None;
    }

    // Parse components - query string becomes part of the qualifier
    let original_remainder = &original_form[4..]; // Skip "ark:"
    let mut original_parts = original_remainder.splitn(2, '/');
    let naan = original_parts.next()?.to_string();
    let rest = original_parts.next()?;

    // Extract shoulder from the part before query string
    let rest_without_query = rest.split('?').next().unwrap_or(rest);
    let shoulder = extract_shoulder(rest_without_query)?.to_string();

    // Extract blade (without query string) and qualifier (with query string)
    let after_shoulder = &rest[shoulder.len()..];

    // Find where the blade ends (either at '/' or '?')
    let blade_end = after_shoulder
        .find('/')
        .or_else(|| after_shoulder.find('?'));

    let (blade, qualifier) = if let Some(end_pos) = blade_end {
        let blade = after_shoulder[..end_pos].to_string();
        let qualifier_start = if after_shoulder.as_bytes()[end_pos] == b'/' {
            end_pos + 1 // Skip the '/'
        } else {
            end_pos // Keep the '?' as part of qualifier
        };
        (blade, after_shoulder[qualifier_start..].to_string())
    } else {
        (after_shoulder.to_string(), String::new())
    };

    // Get fully normalized version for comparison
    let normalized_ark = normalize_ark_string(ark);

    Some(Ark {
        original: original_form,
        naan,
        shoulder,
        blade,
        qualifier,
        normalized_ark,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shoulder_extraction() {
        assert_eq!(extract_shoulder("x6np1wh8k"), Some("x6"));
        assert_eq!(extract_shoulder("b3test"), Some("b3"));
        assert_eq!(extract_shoulder("abc7def"), Some("abc7"));
        assert_eq!(extract_shoulder("xyz"), None); // No digit
    }

    #[test]
    fn test_ark_parsing() {
        let ark = "ark:12345/x6np1wh8k/nl7l/page2.pdf";
        let parsed = parse_ark(ark).unwrap();

        assert_eq!(parsed.naan, "12345");
        assert_eq!(parsed.shoulder, "x6");
        assert_eq!(parsed.blade, "np1wh8k");
        assert_eq!(parsed.qualifier, "nl7l/page2.pdf");
    }

    #[test]
    fn test_ark_parsing_both_formats() {
        let modern = parse_ark("ark:12345/x6np1wh8k").unwrap();
        let classic = parse_ark("ark:/12345/x6np1wh8k").unwrap();

        assert_eq!(modern, classic);
    }

    #[test]
    fn test_hyphen_removal() {
        // Per RFC 3.1: hyphens are identity-inert for COMPARISON
        // But we store the original form for resolution
        let with_hyphens = parse_ark("ark:12345/x5-4-xz-321").unwrap();
        let without_hyphens = parse_ark("ark:12345/x54xz321").unwrap();

        // Equality comparison uses normalized form
        assert_eq!(with_hyphens, without_hyphens);

        // But original components are preserved
        assert_eq!(with_hyphens.shoulder, "x5");
        assert_eq!(with_hyphens.blade, "-4-xz-321"); // Hyphens preserved!

        // Without hyphens has clean components
        assert_eq!(without_hyphens.shoulder, "x5");
        assert_eq!(without_hyphens.blade, "4xz321");
    }

    #[test]
    fn test_hyphen_like_character_removal() {
        // Test removal of Unicode hyphen-like characters (U+2010 to U+2015)
        let with_en_dash = parse_ark("ark:12345/x6np–1wh8k").unwrap(); // U+2013 EN DASH
        let with_em_dash = parse_ark("ark:12345/x6np—1wh8k").unwrap(); // U+2014 EM DASH
        let normal = parse_ark("ark:12345/x6np1wh8k").unwrap();

        assert_eq!(with_en_dash, normal);
        assert_eq!(with_em_dash, normal);
    }

    #[test]
    fn test_query_string_removal() {
        // Per RFC 3.2: query strings must be removed during normalization FOR COMPARISON ONLY
        let with_query = parse_ark("ark:12345/x6np1wh8k?foo=bar&baz=qux").unwrap();
        let without_query = parse_ark("ark:12345/x6np1wh8k").unwrap();

        // Equal for comparison (normalized form strips query string)
        assert_eq!(with_query, without_query);

        // But original preserves query string
        assert_eq!(with_query.original, "ark:12345/x6np1wh8k?foo=bar&baz=qux");
        assert_eq!(without_query.original, "ark:12345/x6np1wh8k");
    }

    #[test]
    fn test_query_string_with_qualifier() {
        let with_query = parse_ark("ark:12345/x6np1wh8k/page2?foo=bar").unwrap();
        let without_query = parse_ark("ark:12345/x6np1wh8k/page2").unwrap();

        // Equal for comparison
        assert_eq!(with_query, without_query);

        // Qualifier includes query string (for forwarding during resolution)
        assert_eq!(with_query.qualifier, "page2?foo=bar");

        // But original also includes query string
        assert_eq!(with_query.original, "ark:12345/x6np1wh8k/page2?foo=bar");
    }

    #[test]
    fn test_query_string_without_path_qualifier() {
        // Query string becomes the qualifier when there's no path
        let with_query = parse_ark("ark:12345/x6np1wh8k?info").unwrap();
        let without_query = parse_ark("ark:12345/x6np1wh8k").unwrap();

        // Equal for comparison (normalized form removes query)
        assert_eq!(with_query, without_query);

        // Components
        assert_eq!(with_query.naan, "12345");
        assert_eq!(with_query.shoulder, "x6");
        assert_eq!(with_query.blade, "np1wh8k");
        // Query string is the qualifier
        assert_eq!(with_query.qualifier, "?info");

        // Without query has empty qualifier
        assert_eq!(without_query.qualifier, "");
    }

    #[test]
    fn test_trailing_slash_removal() {
        // Per RFC 3.2: trailing slashes should be removed
        let with_trailing = parse_ark("ark:12345/x6np1wh8k/").unwrap();
        let without_trailing = parse_ark("ark:12345/x6np1wh8k").unwrap();

        assert_eq!(with_trailing, without_trailing);
    }

    #[test]
    fn test_trailing_period_removal() {
        // Per RFC 3.2: trailing periods should be removed
        let with_trailing = parse_ark("ark:12345/x6np1wh8k.").unwrap();
        let without_trailing = parse_ark("ark:12345/x6np1wh8k").unwrap();

        assert_eq!(with_trailing, without_trailing);
    }

    #[test]
    fn test_trailing_structural_chars_on_qualifier() {
        // Trailing chars are preserved in original, but removed in normalized
        let ark = parse_ark("ark:12345/x6np1wh8k/page2.pdf/").unwrap();
        assert_eq!(ark.qualifier, "page2.pdf/"); // Original preserved

        let ark2 = parse_ark("ark:12345/x6np1wh8k/page2.").unwrap();
        assert_eq!(ark2.qualifier, "page2."); // Original preserved

        // But they're equal to versions without trailing chars (normalized comparison)
        let clean1 = parse_ark("ark:12345/x6np1wh8k/page2.pdf").unwrap();
        let clean2 = parse_ark("ark:12345/x6np1wh8k/page2").unwrap();
        assert_eq!(ark, clean1);
        assert_eq!(ark2, clean2);
    }

    #[test]
    fn test_naan_lowercase_normalization() {
        // Per RFC 3.2: NAAN should be normalized to lowercase FOR COMPARISON
        // But we store the original case for resolution
        let uppercase_naan = parse_ark("ark:ABCDE/x6np1wh8k").unwrap();
        let lowercase_naan = parse_ark("ark:abcde/x6np1wh8k").unwrap();
        let mixed_case = parse_ark("ark:AbCdE/x6np1wh8k").unwrap();

        // Equality uses normalized form
        assert_eq!(uppercase_naan, lowercase_naan);
        assert_eq!(uppercase_naan, mixed_case);

        // But original case is preserved in fields
        assert_eq!(uppercase_naan.naan, "ABCDE");
        assert_eq!(lowercase_naan.naan, "abcde");
        assert_eq!(mixed_case.naan, "AbCdE");
    }

    #[test]
    fn test_combined_normalization() {
        // Test multiple normalization features together
        // This simulates an ARK that was copy-pasted from formatted text
        let messy = parse_ark("ark:/ABCDE/x6-np-1wh8k/page2.pdf/?foo=bar").unwrap();
        let clean = parse_ark("ark:abcde/x6np1wh8k/page2.pdf").unwrap();

        // They're equal for comparison (normalized)
        assert_eq!(messy, clean);

        // But messy ARK preserves original components (WITH query string in qualifier)
        assert_eq!(messy.naan, "ABCDE"); // Original case preserved
        assert_eq!(messy.shoulder, "x6");
        assert_eq!(messy.blade, "-np-1wh8k"); // Hyphens preserved
        assert_eq!(messy.qualifier, "page2.pdf/?foo=bar"); // Trailing slash AND query preserved

        // Clean ARK has normalized components
        assert_eq!(clean.naan, "abcde");
        assert_eq!(clean.shoulder, "x6");
        assert_eq!(clean.blade, "np1wh8k");
        assert_eq!(clean.qualifier, "page2.pdf");
    }

    #[test]
    fn test_whitespace_removal() {
        // Per RFC 3.1: normalize whitespace from text wrapping/copy-paste
        let with_spaces = parse_ark("ark:12345/x6np 1wh8k").unwrap();
        let with_newline = parse_ark("ark:12345/x6np\n1wh8k").unwrap();
        let with_tab = parse_ark("ark:12345/x6np\t1wh8k").unwrap();
        let clean = parse_ark("ark:12345/x6np1wh8k").unwrap();

        // All should be equal when normalized
        assert_eq!(with_spaces, clean);
        assert_eq!(with_newline, clean);
        assert_eq!(with_tab, clean);

        // But original components preserve whitespace
        assert_eq!(with_spaces.blade, "np 1wh8k");
        assert_eq!(with_newline.blade, "np\n1wh8k");
        assert_eq!(with_tab.blade, "np\t1wh8k");
    }

    #[test]
    fn test_line_wrapped_ark() {
        // Simulate an ARK that was line-wrapped in an email or document
        let wrapped = parse_ark("ark:12345/x6np1wh8k/\npage2.pdf").unwrap();
        let clean = parse_ark("ark:12345/x6np1wh8k/page2.pdf").unwrap();

        assert_eq!(wrapped, clean);
        assert_eq!(wrapped.qualifier, "\npage2.pdf"); // Original preserves newline
    }

    #[test]
    fn test_rfc_example_equivalence() {
        // Per RFC 3.1, these ARKs should be equivalent FOR COMPARISON:
        // ark:12345/x5-4-xz-321
        // https://sneezy.dopey.com/ark:12345/x54--xz32-1
        // ark:12345/x54xz321

        let ark1 = parse_ark("ark:12345/x5-4-xz-321").unwrap();
        let ark2 = parse_ark("ark:12345/x54--xz32-1").unwrap();
        let ark3 = parse_ark("ark:12345/x54xz321").unwrap();

        // All three are equal for comparison
        assert_eq!(ark1, ark2);
        assert_eq!(ark2, ark3);

        // But they preserve their original forms
        assert_eq!(ark1.shoulder, "x5");
        assert_eq!(ark1.blade, "-4-xz-321"); // Hyphens preserved

        assert_eq!(ark2.shoulder, "x5");
        assert_eq!(ark2.blade, "4--xz32-1"); // Hyphens preserved

        assert_eq!(ark3.shoulder, "x5");
        assert_eq!(ark3.blade, "4xz321"); // No hyphens in original
    }
}
