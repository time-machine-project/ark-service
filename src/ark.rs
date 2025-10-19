use crate::AppError;

/// An ARK identifier parsed into its components
#[derive(Debug)]
pub struct Ark {
    /// The complete ark identifier
    pub ark: String,
    /// The NAAN (Name Assigning Authority Number)
    pub naan: String,
    /// The shoulder (prefix) of the ARK
    pub shoulder: String,
    /// The blade (unique identifier) of the ARK
    pub blade: String,
    /// The qualifier (optional additional path) of the ARK
    pub qualifier: String,
}

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

/// Parse an ARK identifier into its components
pub fn parse_ark(ark: &str) -> Option<Ark> {
    // Handle both ark: and ark:/
    let ark_normalized = ark.replace("ark:/", "ark:");

    if !ark_normalized.starts_with("ark:") {
        return None;
    }

    // Remove 'ark:' prefix
    let remainder = &ark_normalized[4..];

    // Split on first /
    let mut parts = remainder.splitn(2, '/');
    let naan = parts.next()?.to_string();
    let rest = parts.next()?;

    // Extract shoulder
    let shoulder = extract_shoulder(rest)?.to_string();

    // Extract blade and qualifier
    let after_shoulder = &rest[shoulder.len()..];
    let mut blade_parts = after_shoulder.splitn(2, '/');
    let blade = blade_parts.next()?.to_string();
    let qualifier = blade_parts.next().unwrap_or("").to_string();

    Some(Ark {
        ark: ark_normalized,
        naan,
        shoulder,
        blade,
        qualifier,
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

        assert_eq!(modern.naan, classic.naan);
        assert_eq!(modern.shoulder, classic.shoulder);
        assert_eq!(modern.blade, classic.blade);
    }
}
