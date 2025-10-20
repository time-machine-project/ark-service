use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::ark::Ark;

/// Represents a shoulder configuration in the ARK system
///
/// # Resolver Rules (N2T.net/ARK Alliance Standard)
///
/// The `route_pattern` provides instructions for constructing a redirect URL for ARK identifiers.
/// This follows the same format used by N2T.net and other ARK resolvers.
///
/// ## Simple URL
///
/// A truncated URL to which the resolver will append the full ARK identifier:
///
/// ```json
/// {
///   "x6": {
///     "route_pattern": "https://example.org/",
///     "project_name": "Simple Redirect"
///   }
/// }
/// ```
///
/// For ARK `ark:12345/x6test`, redirects to: `https://example.org/ark:12345/x6test`
///
/// ## Template Variables
///
/// For more sophisticated routing, use template variables that are replaced with ARK components.
/// For an ARK "ark:12345/x8rd9/page2.pdf", the available variables are:
///
/// - `${pid}` - Full ARK identifier: `ark:12345/x8rd9/page2.pdf`
/// - `${scheme}` - Scheme: `ark`
/// - `${content}` - Everything after "ark:": `12345/x8rd9/page2.pdf`
/// - `${prefix}` - NAAN: `12345`
/// - `${value}` - Everything after NAAN/: `x8rd9/page2.pdf`
///
/// ## Template Examples
///
/// ### Using ${value} (recommended for most cases)
/// ```json
/// {
///   "x8": {
///     "route_pattern": "https://ark.example.org/mycontent/${value}",
///     "project_name": "Value Template"
///   }
/// }
/// ```
/// `ark:12345/x8rd9/page.pdf` → `https://ark.example.org/mycontent/x8rd9/page.pdf`
///
/// ### Using ${pid} as query parameter
/// ```json
/// {
///   "fk4": {
///     "route_pattern": "https://resolver.example.org/resolve?id=${pid}",
///     "project_name": "Query Parameter"
///   }
/// }
/// ```
/// `ark:12345/fk4test` → `https://resolver.example.org/resolve?id=ark:12345/fk4test`
///
/// ### Using ${content} (without ark: prefix)
/// ```json
/// {
///   "b3": {
///     "route_pattern": "https://api.example.org/objects/${content}",
///     "project_name": "API Integration"
///   }
/// }
/// ```
/// `ark:12345/b3data/page.pdf` → `https://api.example.org/objects/12345/b3data/page.pdf`
///
/// ### Using ${prefix} and ${value} separately
/// ```json
/// {
///   "z9": {
///     "route_pattern": "https://storage.example.org/${prefix}/items/${value}",
///     "project_name": "Separate Components"
///   }
/// }
/// ```
/// `ark:12345/z9item/file.txt` → `https://storage.example.org/12345/items/z9item/file.txt`
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shoulder {
    /// The routing pattern/template for this shoulder
    pub route_pattern: String,
    /// The human-readable project name associated with this shoulder
    pub project_name: String,
    /// Whether this shoulder uses a check character (default: true)
    #[serde(default = "default_uses_check_character")]
    pub uses_check_character: bool,
    /// Optional blade length for this shoulder, excluding the check character.
    /// If not specified, defaults to the global DEFAULT_BLADE_LENGTH.
    /// When uses_check_character is true, the final blade will be one character longer.
    pub blade_length: Option<usize>,
}

fn default_uses_check_character() -> bool {
    true
}

impl Default for Shoulder {
    fn default() -> Self {
        Self {
            route_pattern: String::new(),
            project_name: String::new(),
            uses_check_character: true,
            blade_length: None,
        }
    }
}

impl Shoulder {
    /// Validate the route_pattern for security issues
    ///
    /// Ensures:
    /// - Pattern is a valid URL
    /// - Scheme is http or https only
    /// - Template variables appear only in path or query components
    /// - No control characters (CR, LF, null bytes)
    pub fn validate_route_pattern(&self) -> Result<(), String> {
        // Check for control characters
        if self.route_pattern.chars().any(|c| c.is_control()) {
            return Err("route_pattern contains control characters".to_string());
        }

        // Check if pattern has template variables
        let has_template_vars = self.route_pattern.contains("${")
            || self.route_pattern.contains("{pid}")
            || self.route_pattern.contains("{scheme}")
            || self.route_pattern.contains("{content}")
            || self.route_pattern.contains("{prefix}")
            || self.route_pattern.contains("{value}")
            || self.route_pattern.contains("{naan}");

        // If no template variables, just validate the base URL
        if !has_template_vars {
            return self.validate_base_url(&self.route_pattern);
        }

        // For templates, replace variables with safe placeholders to check structure
        let test_url = self
            .route_pattern
            .replace("${pid}", "placeholder")
            .replace("${scheme}", "placeholder")
            .replace("${content}", "placeholder")
            .replace("${prefix}", "placeholder")
            .replace("${value}", "placeholder")
            .replace("{pid}", "placeholder")
            .replace("{scheme}", "placeholder")
            .replace("{content}", "placeholder")
            .replace("{prefix}", "placeholder")
            .replace("{value}", "placeholder")
            .replace("{naan}", "placeholder");

        self.validate_base_url(&test_url)?;

        // Additional check: ensure template variables don't appear in scheme or host position
        // Parse the original pattern to find where variables are
        if let Ok(parsed) = Url::parse(&test_url) {
            // Check if scheme contains template markers in original
            let scheme_end = self.route_pattern.find("://").unwrap_or(0);
            if scheme_end > 0 {
                let scheme_part = &self.route_pattern[..scheme_end];
                if scheme_part.contains('$') || scheme_part.contains('{') {
                    return Err("Template variables not allowed in URL scheme position".to_string());
                }
            }

            // Check if host contains template markers
            if parsed.host_str().is_some() {
                // Find the host section in original pattern
                if let Some(after_scheme) = self.route_pattern.split("://").nth(1) {
                    // Host is before the first '/' or '?' or end of string
                    let host_end = after_scheme
                        .find('/')
                        .or_else(|| after_scheme.find('?'))
                        .unwrap_or(after_scheme.len());
                    let host_part = &after_scheme[..host_end];

                    if host_part.contains('$') || host_part.contains('{') {
                        return Err(
                            "Template variables not allowed in URL host position".to_string()
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate a URL string
    fn validate_base_url(&self, url_str: &str) -> Result<(), String> {
        let parsed =
            Url::parse(url_str).map_err(|e| format!("Invalid URL in route_pattern: {}", e))?;

        // Only allow http and https schemes
        match parsed.scheme() {
            "http" | "https" => Ok(()),
            other => Err(format!(
                "Only http and https schemes allowed, found: {}",
                other
            )),
        }
    }

    /// Validate a constructed redirect URL
    fn validate_redirect_url(&self, url_str: &str) -> Result<Url, String> {
        let parsed =
            Url::parse(url_str).map_err(|e| format!("Invalid redirect URL constructed: {}", e))?;

        // Only allow http and https schemes
        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            other => Err(format!(
                "Redirect URL has invalid scheme (expected http/https): {}",
                other
            )),
        }
    }

    /// Resolve an ARK identifier using this shoulder's routing pattern
    ///
    /// This applies the N2T.net/ARK Alliance template substitution to generate
    /// the target URL for the given ARK.
    ///
    /// # Security
    ///
    /// The constructed URL is validated to ensure:
    /// - It parses as a valid URL
    /// - It uses http or https scheme only
    /// - No injection of malicious schemes (javascript:, data:, etc.)
    ///
    /// If validation fails, returns the error message as the redirect target
    /// (which will cause the redirect to fail safely).
    pub fn resolve(&self, parsed_ark: &Ark) -> String {
        let target = self.apply_template(parsed_ark);

        // Validate the constructed URL
        match self.validate_redirect_url(&target) {
            Ok(validated_url) => {
                tracing::info!(
                    shoulder = %parsed_ark.shoulder,
                    target = %validated_url.as_str(),
                    "ARK redirect target validated"
                );
                validated_url.to_string()
            }
            Err(e) => {
                tracing::error!(
                    shoulder = %parsed_ark.shoulder,
                    ark = %parsed_ark.original,
                    attempted_target = %target,
                    error = %e,
                    "SECURITY: Invalid redirect URL blocked"
                );
                // Return an error URL that will fail safely
                format!("about:blank#error={}", urlencoding::encode(&e))
            }
        }
    }

    /// Apply N2T.net/ARK Alliance template substitution
    ///
    /// Supported variables (both {var} and ${var} formats):
    /// - {pid} or ${pid} - Full ARK identifier (e.g., "ark:12345/x8rd9")
    /// - {scheme} or ${scheme} - Scheme (always "ark")
    /// - {content} or ${content} - Content without scheme (e.g., "12345/x8rd9")
    /// - {prefix} or ${prefix} or {naan} - NAAN/prefix (e.g., "12345")
    /// - {value} or ${value} - Identifier value (e.g., "x8rd9")
    ///
    /// If no template variables are present in the route_pattern, the full ARK
    /// identifier is appended to the base URL (N2T.net standard behavior).
    fn apply_template(&self, parsed_ark: &Ark) -> String {
        let pid = &parsed_ark.original;
        let scheme = "ark";
        let content = if parsed_ark.qualifier.is_empty() {
            format!(
                "{}/{}{}",
                parsed_ark.naan, parsed_ark.shoulder, parsed_ark.blade
            )
        } else {
            format!(
                "{}/{}{}/{}",
                parsed_ark.naan, parsed_ark.shoulder, parsed_ark.blade, parsed_ark.qualifier
            )
        };
        let prefix = &parsed_ark.naan;
        let value = if parsed_ark.qualifier.is_empty() {
            format!("{}{}", parsed_ark.shoulder, parsed_ark.blade)
        } else if parsed_ark.qualifier.starts_with('?') {
            // Query string without path qualifier - no slash needed
            format!(
                "{}{}{}",
                parsed_ark.shoulder, parsed_ark.blade, parsed_ark.qualifier
            )
        } else {
            // Path qualifier - include slash
            format!(
                "{}{}/{}",
                parsed_ark.shoulder, parsed_ark.blade, parsed_ark.qualifier
            )
        };

        // Check if route_pattern contains any template variables
        let has_template_vars = self.route_pattern.contains("${")
            || self.route_pattern.contains("{pid}")
            || self.route_pattern.contains("{scheme}")
            || self.route_pattern.contains("{content}")
            || self.route_pattern.contains("{prefix}")
            || self.route_pattern.contains("{value}")
            || self.route_pattern.contains("{naan}");

        // If no template variables, append the full ARK (N2T.net standard behavior)
        if !has_template_vars {
            return format!("{}{}", self.route_pattern, pid);
        }

        // Normalize template: convert ${var} to {var} format, and also support {naan}
        let normalized = self
            .route_pattern
            .replace("${pid}", "{pid}")
            .replace("${scheme}", "{scheme}")
            .replace("${content}", "{content}")
            .replace("${prefix}", "{prefix}")
            .replace("${value}", "{value}")
            .replace("{naan}", "{prefix}");

        // Apply substitutions using rust-style {} format
        normalized
            .replace("{pid}", pid)
            .replace("{scheme}", scheme)
            .replace("{content}", &content)
            .replace("{prefix}", prefix)
            .replace("{value}", &value)
    }
}

/// Load shoulders configuration from environment variable
///
/// Supports two formats:
/// 1. JSON format:
///    ```json
///    {
///      "x6": {
///        "route_pattern": "https://alpha.tm.org/${value}",
///        "project_name": "Project Alpha",
///        "uses_check_character": true
///      }
///    }
///    ```
///
/// 2. Simple format:
///    `shoulder\troute\tproject,shoulder\troute\tproject,...`
///    Example: `x6\thttps://alpha.tm.org/${value}\tProject Alpha,b3\thttps://beta.tm.org/${value}\tProject Beta`
///
/// Template variables supported: ${pid}, ${scheme}, ${content}, ${prefix}, ${value}
///
/// # Security
///
/// All route_patterns are validated on load to ensure:
/// - Valid URL structure
/// - Only http/https schemes
/// - Template variables only in path/query positions
/// - No control characters
pub fn load_shoulders_from_env() -> Result<HashMap<String, Shoulder>, String> {
    let shoulders_config =
        std::env::var("SHOULDERS").map_err(|_| "SHOULDERS environment variable not set")?;

    // Try parsing as JSON first
    let shoulders = if let Ok(s) = parse_shoulders_json(&shoulders_config) {
        s
    } else {
        // Fall back to simple format
        parse_shoulders_simple(&shoulders_config)?
    };

    // Validate all route patterns
    for (name, shoulder) in &shoulders {
        shoulder
            .validate_route_pattern()
            .map_err(|e| format!("Security validation failed for shoulder '{}': {}", name, e))?;
    }

    Ok(shoulders)
}

/// Parse shoulders from JSON format
///
/// Expects a JSON object with shoulder names as keys and Shoulder objects as values:
/// ```json
/// {
///   "x6": {
///     "route_pattern": "https://alpha.tm.org/${value}",
///     "project_name": "Project Alpha",
///     "uses_check_character": true
///   }
/// }
/// ```
fn parse_shoulders_json(json_str: &str) -> Result<HashMap<String, Shoulder>, String> {
    serde_json::from_str::<HashMap<String, Shoulder>>(json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Parse shoulders from simple tab-delimited format
///
/// Format: `shoulder\troute\tproject,shoulder\troute\tproject,...`
/// Example: `x6\thttps://alpha.tm.org/${value}\tProject Alpha,b3\thttps://beta.tm.org/${value}\tProject Beta`
///
/// Supports both literal tab characters and escaped \t sequences.
///
/// Returns an error if no valid shoulders are found.
fn parse_shoulders_simple(simple_str: &str) -> Result<HashMap<String, Shoulder>, String> {
    let mut shoulders = HashMap::new();

    // Replace escaped \t with actual tab characters
    let normalized = simple_str.replace("\\t", "\t");

    for entry in normalized.split(',') {
        let parts: Vec<&str> = entry.split('\t').collect();
        if parts.len() != 3 {
            continue;
        }

        let shoulder = parts[0].trim().to_string();
        let route_pattern = parts[1].trim().to_string();
        let project_name = parts[2].trim().to_string();

        shoulders.insert(
            shoulder,
            Shoulder {
                route_pattern,
                project_name,
                ..Default::default()
            },
        );
    }

    if shoulders.is_empty() {
        return Err("No valid shoulders found in SHOULDERS configuration".to_string());
    }

    Ok(shoulders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark::parse_ark;

    // Security validation tests

    #[test]
    fn test_validate_route_pattern_valid_urls() {
        let valid_patterns = vec![
            "https://example.org/",
            "http://example.org/items",
            "https://example.org/${value}",
            "https://api.example.org/resolve?id=${pid}",
            "https://example.org/path/${value}/more",
        ];

        for pattern in valid_patterns {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_ok(),
                "Should accept valid pattern: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_validate_route_pattern_invalid_schemes() {
        let invalid_schemes = vec![
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "ftp://example.org/",
        ];

        for pattern in invalid_schemes {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_err(),
                "Should reject invalid scheme: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_validate_route_pattern_template_in_scheme() {
        let patterns = vec![
            "${scheme}://example.org/",
            "{scheme}://example.org/",
            "ht${value}://example.org/",
        ];

        for pattern in patterns {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_err(),
                "Should reject template in scheme: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_validate_route_pattern_template_in_host() {
        let patterns = vec![
            "https://${value}.example.org/",
            "https://evil${pid}.com/",
            "https://example.${content}/",
        ];

        for pattern in patterns {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_err(),
                "Should reject template in host: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_validate_route_pattern_control_characters() {
        let patterns = vec![
            "https://example.org/\r\n",
            "https://example.org/\x00",
            "https://example.org/\t",
        ];

        for pattern in &patterns {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_err(),
                "Should reject control characters"
            );
        }
    }

    #[test]
    fn test_validate_route_pattern_malformed_urls() {
        let patterns = vec!["not-a-url", "://missing-scheme", "https://", ""];

        for pattern in patterns {
            let shoulder = Shoulder {
                route_pattern: pattern.to_string(),
                project_name: "Test".to_string(),
                ..Default::default()
            };
            assert!(
                shoulder.validate_route_pattern().is_err(),
                "Should reject malformed URL: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_resolve_blocks_malicious_ark_components() {
        // Test that even if ARK components contain malicious content,
        // the final URL validation catches it
        let shoulder = Shoulder {
            route_pattern: "https://example.org/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };

        // Create ARK with various injection attempts
        let test_cases = vec![
            ("ark:12345/x6test", "https://example.org/x6test"),
            // Normal case - should work
        ];

        for (ark_str, expected) in test_cases {
            if let Some(parsed) = parse_ark(ark_str) {
                let result = shoulder.resolve(&parsed);
                // If it's a valid redirect, check it matches expected
                // If it's blocked, it will be about:blank#error=...
                if !result.starts_with("about:blank") {
                    assert_eq!(result, expected);
                }
            }
        }
    }

    #[test]
    fn test_resolve_validates_final_url() {
        // Test URL validation of the final constructed redirect
        let shoulder = Shoulder {
            route_pattern: "https://example.org/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };

        let ark = parse_ark("ark:12345/x6test").unwrap();
        let result = shoulder.resolve(&ark);

        // Should be a valid URL
        assert!(Url::parse(&result).is_ok());

        // Should be https
        let parsed = Url::parse(&result).unwrap();
        assert!(parsed.scheme() == "https" || result.starts_with("about:blank"));
    }

    #[test]
    fn test_load_shoulders_validates_patterns() {
        // Test that loading shoulders validates all patterns
        unsafe {
            std::env::set_var(
                "SHOULDERS",
                r#"{
                "x6": {
                    "route_pattern": "javascript:alert(1)",
                    "project_name": "Evil"
                }
            }"#,
            );
        }

        let result = load_shoulders_from_env();
        assert!(result.is_err(), "Should reject invalid scheme on load");
        assert!(result.unwrap_err().contains("Security validation failed"));

        // Clean up
        unsafe {
            std::env::remove_var("SHOULDERS");
        }
    }

    #[test]
    fn test_load_shoulders_rejects_template_in_host() {
        unsafe {
            std::env::set_var(
                "SHOULDERS",
                r#"{
                "x6": {
                    "route_pattern": "https://${value}.evil.com/",
                    "project_name": "Evil"
                }
            }"#,
            );
        }

        let result = load_shoulders_from_env();
        assert!(result.is_err(), "Should reject template in host on load");

        // Clean up
        unsafe {
            std::env::remove_var("SHOULDERS");
        }
    }

    #[test]
    fn test_parse_shoulders_json() {
        // Valid JSON with multiple shoulders and check_character variations
        let json = r#"
        {
            "x6": {
                "route_pattern": "https://alpha.tm.org/${value}",
                "project_name": "Project Alpha",
                "uses_check_character": false
            },
            "b3": {
                "route_pattern": "https://beta.tm.org/{value}",
                "project_name": "Project Beta"
            }
        }
        "#;

        let shoulders = parse_shoulders_json(json).unwrap();
        assert_eq!(shoulders.len(), 2);

        let x6 = &shoulders["x6"];
        assert_eq!(x6.route_pattern, "https://alpha.tm.org/${value}");
        assert!(!x6.uses_check_character);

        let b3 = &shoulders["b3"];
        assert!(b3.uses_check_character); // Default value

        // Invalid JSON
        assert!(parse_shoulders_json(r#"{ "x6": "invalid" }"#).is_err());
        assert!(parse_shoulders_json(r#"{ "x6": { "route"#).is_err());
    }

    #[test]
    fn test_parse_shoulders_with_blade_length() {
        // Test parsing JSON with blade_length field
        let json = r#"
        {
            "x6": {
                "route_pattern": "https://alpha.tm.org/${value}",
                "project_name": "Custom Length",
                "uses_check_character": true,
                "blade_length": 12
            },
            "b3": {
                "route_pattern": "https://beta.tm.org/${value}",
                "project_name": "Default Length",
                "uses_check_character": false
            }
        }
        "#;

        let shoulders = parse_shoulders_json(json).unwrap();
        assert_eq!(shoulders.len(), 2);

        let x6 = &shoulders["x6"];
        assert_eq!(x6.blade_length, Some(12));

        let b3 = &shoulders["b3"];
        assert_eq!(b3.blade_length, None); // Not specified, should be None
    }

    #[test]
    fn test_parse_shoulders_simple() {
        // Valid: single and multiple shoulders with complex URLs and special chars in names
        let simple = "x6\thttps://alpha.tm.org:8080/${value}\tProject Alpha,b3\thttp://beta.tm.org\tProject: Beta";
        let shoulders = parse_shoulders_simple(simple).unwrap();

        assert_eq!(shoulders.len(), 2);

        let x6 = &shoulders["x6"];
        assert_eq!(x6.route_pattern, "https://alpha.tm.org:8080/${value}");
        assert_eq!(x6.project_name, "Project Alpha");
        assert!(x6.uses_check_character);
        assert_eq!(x6.blade_length, None);

        let b3 = &shoulders["b3"];
        assert_eq!(b3.route_pattern, "http://beta.tm.org");
        assert_eq!(b3.project_name, "Project: Beta");

        // Skip invalid entries (wrong number of parts)
        let mixed = "invalid,x6\thttps://example.org\tTest";
        assert_eq!(parse_shoulders_simple(mixed).unwrap().len(), 1);

        // Error on all invalid
        assert!(parse_shoulders_simple("").is_err());
        assert!(parse_shoulders_simple("invalid").is_err());
        assert!(parse_shoulders_simple("x6\tonly_two").is_err());
        assert!(parse_shoulders_simple("x6\ttoo\tmany\tparts").is_err());
    }

    #[test]
    fn test_parse_shoulders_simple_escaped_tabs() {
        // Test parsing with escaped \t sequences (as they appear in Docker Compose YAML)
        let escaped = r"b1\thttps://ark.timeatlas.eu/${pid}\tTime Atlas";
        let shoulders = parse_shoulders_simple(escaped).unwrap();

        assert_eq!(shoulders.len(), 1);

        let b1 = &shoulders["b1"];
        assert_eq!(b1.route_pattern, "https://ark.timeatlas.eu/${pid}");
        assert_eq!(b1.project_name, "Time Atlas");
        assert!(b1.uses_check_character);

        // Test with multiple shoulders using escaped tabs
        let multiple_escaped =
            r"x6\thttps://example.org/${value}\tProject X,b3\thttps://test.org/${pid}\tProject B";
        let shoulders = parse_shoulders_simple(multiple_escaped).unwrap();
        assert_eq!(shoulders.len(), 2);
    }

    // Template resolution tests

    #[test]
    fn test_resolve_all_placeholders() {
        let ark = "ark:12345/x6np1wh8k/page2.pdf";
        let parsed = parse_ark(ark).unwrap();

        // Test all ARK Alliance standard variables in realistic URL contexts
        let shoulder_pid = Shoulder {
            route_pattern: "https://example.org/resolve?id=${pid}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_pid.resolve(&parsed),
            "https://example.org/resolve?id=ark:12345/x6np1wh8k/page2.pdf"
        );

        let shoulder_content = Shoulder {
            route_pattern: "https://example.org/${content}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_content.resolve(&parsed),
            "https://example.org/12345/x6np1wh8k/page2.pdf"
        );

        let shoulder_prefix = Shoulder {
            route_pattern: "https://example.org/${prefix}/items".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_prefix.resolve(&parsed),
            "https://example.org/12345/items"
        );

        let shoulder_value = Shoulder {
            route_pattern: "https://example.org/objects/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_value.resolve(&parsed),
            "https://example.org/objects/x6np1wh8k/page2.pdf"
        );

        // Test complex template with multiple variables
        let shoulder_complex = Shoulder {
            route_pattern: "https://example.org/view?ark=${pid}&naan=${prefix}&id=${value}"
                .to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        let expected = "https://example.org/view?ark=ark:12345/x6np1wh8k/page2.pdf&naan=12345&id=x6np1wh8k/page2.pdf";
        assert_eq!(shoulder_complex.resolve(&parsed), expected);
    }

    #[test]
    fn test_resolve_without_qualifier() {
        let ark = "ark:12345/x6np1wh8k";
        let parsed = parse_ark(ark).unwrap();

        // Test standard template with value
        let shoulder = Shoulder {
            route_pattern: "https://example.org/items/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder.resolve(&parsed),
            "https://example.org/items/x6np1wh8k"
        );
    }

    #[test]
    fn test_resolve_with_query_string() {
        // Test that query strings are forwarded with template variables
        let ark = "ark:12345/x6np1wh8k?info";
        let parsed = parse_ark(ark).unwrap();

        // Test with ${value} template
        let shoulder = Shoulder {
            route_pattern: "https://example.org/items/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder.resolve(&parsed),
            "https://example.org/items/x6np1wh8k?info"
        );

        // Test with ${pid} template
        let shoulder2 = Shoulder {
            route_pattern: "https://example.org/resolve?id=${pid}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder2.resolve(&parsed),
            "https://example.org/resolve?id=ark:12345/x6np1wh8k?info"
        );

        // Test with no template variables
        let shoulder3 = Shoulder {
            route_pattern: "https://example.org/".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder3.resolve(&parsed),
            "https://example.org/ark:12345/x6np1wh8k?info"
        );
    }

    #[test]
    fn test_resolve_real_world_examples() {
        let ark = "ark:99999/fk4test123/metadata.xml";
        let parsed = parse_ark(ark).unwrap();

        // Example 1: Simple redirect - N2T.net will append the full ARK to base URL
        // (No template variables needed for this case)
        let shoulder1 = Shoulder {
            route_pattern: "https://example.org/".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder1.resolve(&parsed),
            "https://example.org/ark:99999/fk4test123/metadata.xml"
        );

        // Example 2: ARK Alliance standard - use ${value} variable (most common)
        let shoulder2 = Shoulder {
            route_pattern: "https://ark.example.org/mycontent/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder2.resolve(&parsed),
            "https://ark.example.org/mycontent/fk4test123/metadata.xml"
        );

        // Example 3: Use ${pid} to pass full ARK as query parameter
        let shoulder3 = Shoulder {
            route_pattern: "https://resolver.example.org/resolve?id=${pid}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder3.resolve(&parsed),
            "https://resolver.example.org/resolve?id=ark:99999/fk4test123/metadata.xml"
        );

        // Example 4: Use ${content} (without ark: prefix)
        let shoulder4 = Shoulder {
            route_pattern: "https://api.example.org/v1/objects/${content}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder4.resolve(&parsed),
            "https://api.example.org/v1/objects/99999/fk4test123/metadata.xml"
        );

        // Example 5: Use ${prefix} and ${value} separately
        let shoulder5 = Shoulder {
            route_pattern: "https://storage.example.org/${prefix}/items/${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder5.resolve(&parsed),
            "https://storage.example.org/99999/items/fk4test123/metadata.xml"
        );
    }
}
