use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Resolve an ARK identifier using this shoulder's routing pattern
    ///
    /// This applies the N2T.net/ARK Alliance template substitution to generate
    /// the target URL for the given ARK.
    pub fn resolve(&self, parsed_ark: &Ark) -> String {
        self.apply_template(parsed_ark)
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
        // Extract ARK components
        let pid = &parsed_ark.ark;
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
        } else {
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
pub fn load_shoulders_from_env() -> Result<HashMap<String, Shoulder>, String> {
    let shoulders_config =
        std::env::var("SHOULDERS").map_err(|_| "SHOULDERS environment variable not set")?;

    // Try parsing as JSON first
    if let Ok(shoulders) = parse_shoulders_json(&shoulders_config) {
        return Ok(shoulders);
    }

    // Fall back to simple format
    parse_shoulders_simple(&shoulders_config)
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

        // Test all ARK Alliance standard variables
        let shoulder_pid = Shoulder {
            route_pattern: "${pid}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_pid.resolve(&parsed),
            "ark:12345/x6np1wh8k/page2.pdf"
        );

        let shoulder_scheme = Shoulder {
            route_pattern: "${scheme}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(shoulder_scheme.resolve(&parsed), "ark");

        let shoulder_content = Shoulder {
            route_pattern: "${content}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(
            shoulder_content.resolve(&parsed),
            "12345/x6np1wh8k/page2.pdf"
        );

        let shoulder_prefix = Shoulder {
            route_pattern: "${prefix}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(shoulder_prefix.resolve(&parsed), "12345");

        let shoulder_value = Shoulder {
            route_pattern: "${value}".to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        };
        assert_eq!(shoulder_value.resolve(&parsed), "x6np1wh8k/page2.pdf");

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
