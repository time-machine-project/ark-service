use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use url::Url;

use crate::ark::{Ark, is_primordinal_shoulder};
use crate::error::AppError;

/// A shoulder's resolver configuration.
///
/// `route_pattern` is a URL to which the ARK is appended, or a template whose variables carry
/// the parts of the ARK as received, with the label as "ark:"; the ARK's query string joins
/// the target's query. For `ark:12345/x8rd9/page2.pdf`:
///
/// - `${pid}`: `ark:12345/x8rd9/page2.pdf`
/// - `${scheme}`: `ark`
/// - `${content}`: `12345/x8rd9/page2.pdf`
/// - `${prefix}` or `${naan}`: `12345`
/// - `${value}`: `x8rd9/page2.pdf`
///
/// Each `${var}` may also be written `{var}`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shoulder {
    pub route_pattern: String,
    pub project_name: String,
    #[serde(default = "default_uses_check_character")]
    pub uses_check_character: bool,
    /// Blade length without the check character; `None` uses `AppState::default_blade_length`.
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
            uses_check_character: default_uses_check_character(),
            blade_length: None,
        }
    }
}

#[derive(Clone, Copy)]
enum Part {
    Pid,
    Scheme,
    Content,
    Prefix,
    Value,
}

const VARIABLES: &[(&str, Part)] = &[
    ("pid", Part::Pid),
    ("scheme", Part::Scheme),
    ("content", Part::Content),
    ("prefix", Part::Prefix),
    ("naan", Part::Prefix),
    ("value", Part::Value),
];

/// Lowercase ASCII, so host normalization leaves it intact and a search finds it in any URL
/// component.
const SENTINEL: &str = "arkplaceholder";

impl Shoulder {
    /// Rejects patterns that are not http(s) URLs, contain control characters, malformed or
    /// unknown variables or dot segments, or let the ARK reach the scheme, user info, host or
    /// port.
    pub fn validate_route_pattern(&self) -> Result<(), String> {
        if self.route_pattern.chars().any(char::is_control) {
            return Err("route_pattern contains control characters".to_string());
        }
        let template = Template::parse(&self.route_pattern)?;
        let target = template.render(|_| SENTINEL);
        if has_dot_segment(&target) {
            return Err("route_pattern contains a '.' or '..' path segment".to_string());
        }
        let url = parse_http_url(&target)?;
        let authority = [
            url.username(),
            url.password().unwrap_or(""),
            url.host_str().unwrap_or(""),
        ];
        if authority.iter().any(|part| part.contains(SENTINEL)) {
            return Err(if template.appends_ark {
                "The appended ARK would change the host; end route_pattern with '/' or '='"
            } else {
                "Template variables are only allowed in the path, query or fragment"
            }
            .to_string());
        }
        Ok(())
    }

    /// Builds the redirect target for an ARK of this shoulder.
    ///
    /// The ARK's query string joins the target's query, so a pattern with text after a
    /// variable keeps that text in place.
    pub fn resolve(&self, ark: &Ark) -> Result<String, AppError> {
        let invalid_target = |e: String| {
            tracing::error!(ark = ?ark.received(), error = %e, "Invalid redirect target");
            AppError::InvalidTarget
        };

        let template = Template::parse(&self.route_pattern).map_err(invalid_target)?;
        let target = template.render(|part| match part {
            Part::Pid => ark.pid(),
            Part::Scheme => "ark",
            Part::Content => ark.content(),
            Part::Prefix => ark.prefix(),
            Part::Value => ark.value(),
        });
        if has_dot_segment(&target) {
            return Err(AppError::InvalidArk);
        }

        let mut url = parse_http_url(&target).map_err(invalid_target)?;
        if let Some(ark_query) = ark.query() {
            let query = match url.query() {
                Some(own) if !own.is_empty() && !ark_query.is_empty() => {
                    format!("{own}&{ark_query}")
                }
                Some(own) if !own.is_empty() => own.to_string(),
                _ => ark_query.to_string(),
            };
            url.set_query(Some(&query));
        }
        Ok(url.into())
    }
}

enum Segment<'a> {
    Literal(&'a str),
    Variable(Part),
}

/// A route pattern split into literal text and variables.
struct Template<'a> {
    segments: Vec<Segment<'a>>,
    appends_ark: bool,
}

impl<'a> Template<'a> {
    /// Every '{' and '}' must belong to a known variable; a pattern without variables gets
    /// the ARK appended.
    fn parse(pattern: &'a str) -> Result<Self, String> {
        let mut segments = Vec::new();
        let mut rest = pattern;
        while let Some(open) = rest.find(['{', '}']) {
            if rest.as_bytes()[open] == b'}' {
                return Err("route_pattern has a '}' without '{'".to_string());
            }
            let after = &rest[open + 1..];
            let close = after
                .find('}')
                .ok_or_else(|| "route_pattern has a '{' without '}'".to_string())?;
            let name = &after[..close];
            let &(_, part) = VARIABLES
                .iter()
                .find(|(known, _)| *known == name)
                .ok_or_else(|| format!("Unknown template variable {{{name}}}"))?;
            let literal = &rest[..open];
            segments.push(Segment::Literal(
                literal.strip_suffix('$').unwrap_or(literal),
            ));
            segments.push(Segment::Variable(part));
            rest = &after[close + 1..];
        }
        segments.push(Segment::Literal(rest));

        let appends_ark = !segments.iter().any(|s| matches!(s, Segment::Variable(_)));
        if appends_ark {
            segments.push(Segment::Variable(Part::Pid));
        }
        Ok(Self {
            segments,
            appends_ark,
        })
    }

    /// Values in the query or fragment have `&`, `=`, `+` and `#` percent-encoded, so the
    /// parts of the ARK cannot add parameters.
    fn render<'v>(&self, value: impl Fn(Part) -> &'v str) -> String {
        let mut target = String::new();
        let mut in_query = false;
        for segment in &self.segments {
            match segment {
                Segment::Literal(literal) => {
                    in_query |= literal.contains(['?', '#']);
                    target.push_str(literal);
                }
                Segment::Variable(part) if in_query => {
                    for c in value(*part).chars() {
                        match c {
                            '&' => target.push_str("%26"),
                            '=' => target.push_str("%3D"),
                            '+' => target.push_str("%2B"),
                            '#' => target.push_str("%23"),
                            c => target.push(c),
                        }
                    }
                }
                Segment::Variable(part) => target.push_str(value(*part)),
            }
        }
        target
    }
}

/// Whether the URL's path has a "." or ".." segment, which URL serialization or the target
/// would resolve away; encoded dots and separators count.
fn has_dot_segment(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.to_ascii_lowercase()
        .replace("%2e", ".")
        .replace("%2f", "/")
        .replace("%5c", "\\")
        .split(['/', '\\'])
        .any(|segment| segment == "." || segment == "..")
}

fn parse_http_url(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL {url:?}: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        other => Err(format!(
            "Only http and https URLs are allowed, found {other:?}"
        )),
    }
}

/// Parses `SHOULDERS`: a JSON object of shoulders, or comma-separated
/// `shoulder\troute_pattern\tproject_name` entries.
pub fn parse_shoulders(config: &str) -> Result<HashMap<String, Shoulder>, String> {
    let entries = if config.trim_start().starts_with('{') {
        serde_json::from_str::<JsonEntries>(config)
            .map_err(|e| format!("Invalid SHOULDERS JSON: {e}"))?
            .0
    } else {
        parse_shoulders_simple(config)?
    };
    if entries.is_empty() {
        return Err("SHOULDERS defines no shoulders".to_string());
    }

    let mut shoulders = HashMap::new();
    for (name, shoulder) in entries {
        if !is_primordinal_shoulder(&name) {
            return Err(format!(
                "Shoulder {name:?} is not a primordinal shoulder: betanumeric consonants, if any, then one digit"
            ));
        }
        if shoulder.blade_length == Some(0) {
            return Err(format!(
                "Shoulder {name:?}: blade_length must be at least 1"
            ));
        }
        shoulder
            .validate_route_pattern()
            .map_err(|e| format!("Shoulder {name:?}: {e}"))?;
        if shoulders.insert(name.clone(), shoulder).is_some() {
            return Err(format!("Shoulder {name:?} is defined twice"));
        }
    }
    Ok(shoulders)
}

/// The entries of a JSON object in order, duplicates included.
struct JsonEntries(Vec<(String, Shoulder)>);

impl<'de> Deserialize<'de> for JsonEntries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = JsonEntries;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object of shoulders")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsonEntries, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(JsonEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

fn parse_shoulders_simple(config: &str) -> Result<Vec<(String, Shoulder)>, String> {
    // Docker Compose YAML passes "\t" as two characters
    let config = config.replace("\\t", "\t");
    let mut shoulders = Vec::new();
    for entry in config.split(',') {
        let fields: Vec<&str> = entry.split('\t').map(str::trim).collect();
        let [name, route_pattern, project_name] = fields[..] else {
            return Err(format!(
                "SHOULDERS entry {entry:?} needs three tab-separated fields"
            ));
        };
        let shoulder = Shoulder {
            route_pattern: route_pattern.to_string(),
            project_name: project_name.to_string(),
            ..Default::default()
        };
        shoulders.push((name.to_string(), shoulder));
    }
    Ok(shoulders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ark::parse_ark;

    fn shoulder(route_pattern: &str) -> Shoulder {
        Shoulder {
            route_pattern: route_pattern.to_string(),
            project_name: "Test".to_string(),
            ..Default::default()
        }
    }

    fn resolve(route_pattern: &str, ark: &str) -> Result<String, AppError> {
        shoulder(route_pattern).resolve(&parse_ark(ark).unwrap())
    }

    #[test]
    fn accepts_patterns_with_variables_in_path_query_or_fragment() {
        for pattern in [
            "https://example.org/",
            "https://example.org/resolve?id=",
            "http://example.org:8080/items/${value}",
            "https://example.org/resolve?id={pid}",
            "https://example.org/${prefix}/${scheme}/{naan}/${content}",
            "https://example.org/page#${value}",
        ] {
            assert!(
                shoulder(pattern).validate_route_pattern().is_ok(),
                "{pattern}"
            );
        }
    }

    #[test]
    fn rejects_patterns_that_let_the_ark_reach_the_authority() {
        for pattern in [
            "https:${value}",
            "https:/${value}",
            "https://${value}.example.org/",
            "https://{value}@example.org/",
            "https://example.org:{value}/",
            "https://example.org{value}",
            "${scheme}://example.org/",
            "https://example.org",
            "https://example.org:8080",
        ] {
            assert!(
                shoulder(pattern).validate_route_pattern().is_err(),
                "{pattern}"
            );
        }
    }

    #[test]
    fn rejects_invalid_patterns() {
        for pattern in [
            "javascript:alert(1)",
            "ftp://example.org/",
            "not a url",
            "https://example.org/\n",
            "https://example.org/${PID}",
            "https://example.org/{unknown}",
            "https://example.org/{value",
            "https://example.org/${ value}",
            "https://example.org/}/${value}",
            "https://example.org/a/../${value}",
        ] {
            assert!(
                shoulder(pattern).validate_route_pattern().is_err(),
                "{pattern}"
            );
        }
    }

    #[test]
    fn substitutes_every_variable() {
        assert_eq!(
            resolve(
                "https://example.org/${pid}/${scheme}/${content}/${prefix}/{naan}/${value}",
                "ark:12345/x6np1wh8k/page2.pdf"
            ),
            Ok("https://example.org/ark:12345/x6np1wh8k/page2.pdf/ark/12345/x6np1wh8k/page2.pdf/12345/12345/x6np1wh8k/page2.pdf".to_string())
        );
    }

    #[test]
    fn appends_the_ark_to_a_pattern_without_variables() {
        assert_eq!(
            resolve("https://example.org/", "ark:12345/x6np1wh8k?info"),
            Ok("https://example.org/ark:12345/x6np1wh8k?info".to_string())
        );
    }

    #[test]
    fn encodes_query_delimiters_in_query_values() {
        assert_eq!(
            resolve(
                "https://example.org/resolve?id={pid}",
                "ark:12345/x6a&admin=1"
            ),
            Ok("https://example.org/resolve?id=ark:12345/x6a%26admin%3D1".to_string())
        );
        assert_eq!(
            resolve("https://example.org/${value}", "ark:12345/x6a&b=1"),
            Ok("https://example.org/x6a&b=1".to_string())
        );
    }

    #[test]
    fn encodes_query_delimiters_in_appended_arks() {
        assert_eq!(
            resolve("https://example.org/resolve?id=", "ark:12345/x6a&admin=1"),
            Ok("https://example.org/resolve?id=ark:12345/x6a%26admin%3D1".to_string())
        );
    }

    #[test]
    fn merges_the_ark_query_into_the_target_query() {
        for (pattern, ark, target) in [
            (
                "https://example.org/${value}/manifest",
                "ark:12345/x6a?info",
                "https://example.org/x6a/manifest?info",
            ),
            (
                "https://example.org/${value}?format=json",
                "ark:12345/x6a?info",
                "https://example.org/x6a?format=json&info",
            ),
            (
                "https://example.org/resolve?id=${pid}",
                "ark:12345/x6a?info",
                "https://example.org/resolve?id=ark:12345/x6a&info",
            ),
            (
                "https://example.org/",
                "ark:12345/x6a??",
                "https://example.org/ark:12345/x6a??",
            ),
            (
                "https://example.org/",
                "ark:12345/x6a?",
                "https://example.org/ark:12345/x6a?",
            ),
        ] {
            assert_eq!(
                resolve(pattern, ark),
                Ok(target.to_string()),
                "{pattern} {ark}"
            );
        }
    }

    #[test]
    fn rejects_targets_with_dot_segments() {
        for (pattern, ark) in [
            ("https://example.org/${value}", "ark:12345/x6a/../admin"),
            ("https://example.org/${value}", "ark:12345/x6a%2F..%2Fadmin"),
            ("https://example.org/a/${value}../secret", "ark:12345/x6a/"),
        ] {
            assert_eq!(
                resolve(pattern, ark),
                Err(AppError::InvalidArk),
                "{pattern} {ark}"
            );
        }
        assert!(
            resolve(
                "https://example.org/resolve?id=${pid}",
                "ark:12345/x6a/../b"
            )
            .is_ok()
        );
    }

    #[test]
    fn reports_a_target_that_is_not_a_url() {
        assert_eq!(
            resolve("https://example.org:{value}/", "ark:12345/x6a"),
            Err(AppError::InvalidTarget)
        );
    }

    #[test]
    fn parses_json_shoulders() {
        let shoulders = parse_shoulders(
            r#"{
                "x6": {"route_pattern": "https://alpha.org/${value}", "project_name": "Alpha", "uses_check_character": false, "blade_length": 12},
                "b3": {"route_pattern": "https://beta.org/{value}", "project_name": "Beta"}
            }"#,
        )
        .unwrap();

        assert!(!shoulders["x6"].uses_check_character);
        assert_eq!(shoulders["x6"].blade_length, Some(12));
        assert!(shoulders["b3"].uses_check_character);
        assert_eq!(shoulders["b3"].blade_length, None);
    }

    #[test]
    fn parses_simple_shoulders_with_escaped_tabs() {
        let shoulders = parse_shoulders(
            r"b1\thttps://ark.timeatlas.eu/{pid}\tTime Atlas, x6	https://example.org/	Project X",
        )
        .unwrap();

        assert_eq!(
            shoulders["b1"].route_pattern,
            "https://ark.timeatlas.eu/{pid}"
        );
        assert_eq!(shoulders["b1"].project_name, "Time Atlas");
        assert!(shoulders["b1"].uses_check_character);
        assert_eq!(shoulders["x6"].project_name, "Project X");
    }

    #[test]
    fn rejects_invalid_shoulder_configurations() {
        for config in [
            "",
            "{}",
            r#"{"x6": {"route_pattern": "https://e.org/", "project_name": "X",}}"#,
            r#"{"x6": {"route_pattern": "https://e.org/"}}"#,
            r#"{"x6": {"route_pattern": "https://e.org/", "project_name": "X", "uses_check_charcter": false}}"#,
            r#"{"x6": {"route_pattern": "https://e.org/", "project_name": "X", "blade_length": 0}}"#,
            r#"{"x6": {"route_pattern": "https://a.org/", "project_name": "A"}, "x6": {"route_pattern": "https://b.org/", "project_name": "B"}}"#,
            r#"{"x6": {"route_pattern": "javascript:alert(1)", "project_name": "X"}}"#,
            "x6\thttps://e.org/",
            "x6\thttps://e.org/\tA,x6\thttps://e.org/\tB",
            "x6\thttps://e.org/a,b/\tA",
        ] {
            assert!(parse_shoulders(config).is_err(), "{config}");
        }
    }

    #[test]
    fn rejects_shoulders_that_are_not_primordinal() {
        for name in ["B1", "ab1", "b12", "b", "x6x"] {
            let config = format!("{name}\thttps://e.org/\tTest");
            assert!(parse_shoulders(&config).is_err(), "{name}");
        }
        assert!(parse_shoulders("bcd7\thttps://e.org/\tTest").is_ok());
    }
}
