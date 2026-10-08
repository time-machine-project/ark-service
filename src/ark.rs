//! ARK parsing and normalization per draft-kunze-ark-43.

use std::fmt;

/// The betanumeric alphabet (§2.3), in the ordinal order of the NCDA.
pub const BETANUMERIC: &[u8] = b"0123456789bcdfghjkmnpqrstvwxz";

pub const LABEL: &str = "ark:";
const OLD_LABEL: &str = "ark:/";

pub fn is_betanumeric(s: &str) -> bool {
    s.bytes().all(|b| BETANUMERIC.contains(&b))
}

pub fn has_label(s: &str) -> bool {
    starts_with_ignore_case(s, LABEL)
}

/// The string a check character covers (§2): NAAN, '/', shoulder and blade.
pub fn check_zone(naan: &str, shoulder: &str, blade: &str) -> String {
    format!("{naan}/{shoulder}{blade}")
}

/// Whether `s` is a primordinal shoulder (§2.4.1) and nothing else.
pub fn is_primordinal_shoulder(s: &str) -> bool {
    split_primordinal_shoulder(s) == (Some(s), "")
}

/// Why a string is not an ARK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    MissingLabel,
    MissingNaan,
    MissingName,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseError::MissingLabel => "Missing \"ark:\" label",
            ParseError::MissingNaan => "Missing NAAN",
            ParseError::MissingName => "Missing Name after the NAAN",
        })
    }
}

/// Why a parsed ARK does not conform to the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    NaanNotBetanumeric,
    CharacterOutsideRepertoire,
    MalformedPercentEncoding,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Violation::NaanNotBetanumeric => "NAAN contains non-betanumeric characters",
            Violation::CharacterOutsideRepertoire => {
                "Name or qualifier contains characters outside the ARK repertoire"
            }
            Violation::MalformedPercentEncoding => "'%' is not followed by two hex digits",
        })
    }
}

/// An ARK split into its normalized components, keeping the string as received.
///
/// Matching and validation use the normalized components. Forwarding uses the received
/// string after the §3.1 cleanup, with the label written as "ark:" and nothing between the
/// label and the NAAN.
#[derive(Debug, Clone)]
pub struct Ark {
    received: String,
    naan_end: usize,
    path_end: usize,
    naan: String,
    shoulder: Option<String>,
    blade: String,
    qualifier: String,
    normalized: String,
}

impl PartialEq for Ark {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for Ark {}

impl Ark {
    /// The ARK as received from its label on, including any query string.
    pub fn received(&self) -> &str {
        &self.received
    }

    /// The ARK as received from its label on, without the query string.
    pub fn pid(&self) -> &str {
        &self.received[..self.path_end]
    }

    /// Everything after the label up to the query string, as received.
    pub fn content(&self) -> &str {
        &self.received[LABEL.len()..self.path_end]
    }

    /// The NAAN as received.
    pub fn prefix(&self) -> &str {
        &self.received[LABEL.len()..self.naan_end]
    }

    /// Everything after the NAAN and its '/' up to the query string, as received.
    pub fn value(&self) -> &str {
        self.received
            .get(self.naan_end + 1..self.path_end)
            .unwrap_or("")
    }

    /// The query string after the first '?', which carries inflections such as `info`.
    pub fn query(&self) -> Option<&str> {
        split_query(&self.received).1
    }

    pub fn naan(&self) -> &str {
        &self.naan
    }

    /// The primordinal shoulder (§2.4.1), if the Name starts with one.
    pub fn shoulder(&self) -> Option<&str> {
        self.shoulder.as_deref()
    }

    /// The rest of the base Name after the shoulder, or the whole base Name without one.
    pub fn blade(&self) -> &str {
        &self.blade
    }

    /// The `ComponentPath` and `VariantPath` (§2.5), including the leading '/' or '.'.
    pub fn qualifier(&self) -> &str {
        &self.qualifier
    }

    /// The normalized ARK (§3.2), used for lexical equivalence.
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    pub fn check_zone(&self) -> String {
        check_zone(
            &self.naan,
            self.shoulder.as_deref().unwrap_or(""),
            &self.blade,
        )
    }

    /// The first rule of §2.3 and §3.1 this ARK breaks, if any.
    pub fn violation(&self) -> Option<Violation> {
        if !is_betanumeric(&self.naan) {
            return Some(Violation::NaanNotBetanumeric);
        }
        let name = &self.normalized.as_bytes()[LABEL.len() + self.naan.len() + 1..];
        for (i, &b) in name.iter().enumerate() {
            if b == b'%' {
                let escape = name.get(i + 1..i + 3);
                if !escape.is_some_and(|e| e.iter().all(u8::is_ascii_hexdigit)) {
                    return Some(Violation::MalformedPercentEncoding);
                }
            } else if !(b.is_ascii_alphanumeric() || b"=~*+@_$./".contains(&b)) {
                return Some(Violation::CharacterOutsideRepertoire);
            }
        }
        None
    }
}

/// Parse an ARK, with or without an NMA in front of it.
pub fn parse_ark(input: &str) -> Result<Ark, ParseError> {
    let cleaned = clean_transcription(input);
    let from_label = strip_nma(&cleaned).ok_or(ParseError::MissingLabel)?;
    let normalized = normalize(from_label);

    let rest = &normalized[LABEL.len()..];
    if rest.is_empty() {
        return Err(ParseError::MissingNaan);
    }
    let (naan, name) = rest.split_once('/').ok_or(ParseError::MissingName)?;
    let base_end = name.find(['/', '.']).unwrap_or(name.len());
    let (base, qualifier) = name.split_at(base_end);
    let (shoulder, blade) = split_primordinal_shoulder(base);

    let content = &from_label[label_len(from_label)..];
    let received = format!("{LABEL}{}", content.trim_start_matches(['/', '.', '-']));

    Ok(Ark {
        naan_end: naan_end(&received),
        path_end: split_query(&received).0.len(),
        received,
        naan: naan.to_string(),
        shoulder: shoulder.map(str::to_string),
        blade: blade.to_string(),
        qualifier: qualifier.to_string(),
        normalized,
    })
}

/// §3.2 steps 2 to 8, for an ARK that starts with its label.
///
/// Step 7 removes no inflections: the service forwards them to the target, and step 2 has
/// already removed the query string that carries `?info`.
fn normalize(ark: &str) -> String {
    let ark = strip_query(ark);
    let ark = normalize_label(ark);
    let ark = lowercase_naan(&ark);
    let ark = uppercase_percent_escapes(&ark);
    let ark = remove_hyphens(&ark);
    normalize_structural_characters(&ark)
}

/// §3.1: remove whitespace and turn hyphen-like characters into hyphens.
fn clean_transcription(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            if ('\u{2010}'..='\u{2015}').contains(&c) {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// Step 1: remove the NMA, everything up to the '/' of the first "/ark:" in its path.
fn strip_nma(ark: &str) -> Option<&str> {
    if has_label(ark) {
        return Some(ark);
    }
    let path_start = scheme_len(ark).map_or(0, |scheme_len| {
        let host_start = scheme_len + "://".len();
        ark[host_start..]
            .find('/')
            .map_or(ark.len(), |i| host_start + i)
    });
    split_query(&ark[path_start..])
        .0
        .to_ascii_lowercase()
        .find("/ark:")
        .map(|i| &ark[path_start + i + 1..])
}

/// Step 2: remove the query string.
fn strip_query(ark: &str) -> &str {
    split_query(ark).0
}

fn split_query(s: &str) -> (&str, Option<&str>) {
    match s.split_once('?') {
        Some((before, query)) => (before, Some(query)),
        None => (s, None),
    }
}

/// The length of a URI scheme (RFC 3986) followed by "://" at the start of `s`.
fn scheme_len(s: &str) -> Option<usize> {
    let len = s.find("://")?;
    let scheme = &s.as_bytes()[..len];
    let valid = scheme.first().is_some_and(u8::is_ascii_alphabetic)
        && scheme
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(b));
    valid.then_some(len)
}

/// Step 3: turn the first "ark:/" or "ark:", in any case, into "ark:".
fn normalize_label(ark: &str) -> String {
    format!("{LABEL}{}", &ark[label_len(ark)..])
}

/// Step 4: lowercase the NAAN.
fn lowercase_naan(ark: &str) -> String {
    let end = naan_end(ark);
    format!(
        "{LABEL}{}{}",
        ark[LABEL.len()..end].to_ascii_lowercase(),
        &ark[end..]
    )
}

/// Step 5: uppercase the two characters after every '%'.
fn uppercase_percent_escapes(ark: &str) -> String {
    let mut pending = 0u8;
    ark.chars()
        .map(|c| {
            let out = if pending > 0 {
                c.to_ascii_uppercase()
            } else {
                c
            };
            pending = if c == '%' {
                2
            } else {
                pending.saturating_sub(1)
            };
            out
        })
        .collect()
}

/// Step 6: remove all hyphens.
fn remove_hyphens(ark: &str) -> String {
    ark.replace('-', "")
}

/// Step 8: drop initial and final '/' and '.', and collapse runs of them to their first character.
fn normalize_structural_characters(ark: &str) -> String {
    let mut out = String::from(LABEL);
    let mut after_structural = true;
    for c in ark[LABEL.len()..].chars() {
        let structural = c == '/' || c == '.';
        if !(structural && after_structural) {
            out.push(c);
        }
        after_structural = structural;
    }
    if out.len() > LABEL.len() && out.ends_with(['/', '.']) {
        out.pop();
    }
    out
}

/// §2.4.1: one or more betanumeric characters ending in the first digit.
fn split_primordinal_shoulder(base: &str) -> (Option<&str>, &str) {
    for (i, b) in base.bytes().enumerate() {
        if b.is_ascii_digit() {
            return (Some(&base[..=i]), &base[i + 1..]);
        }
        if !BETANUMERIC.contains(&b) {
            break;
        }
    }
    (None, base)
}

/// The length of the label "ark:/" or "ark:" that `ark` starts with.
fn label_len(ark: &str) -> usize {
    if starts_with_ignore_case(ark, OLD_LABEL) {
        OLD_LABEL.len()
    } else {
        LABEL.len()
    }
}

/// The index of the '/' that ends the NAAN of an ARK starting with "ark:", or its length.
fn naan_end(ark: &str) -> usize {
    split_query(&ark[LABEL.len()..])
        .0
        .find('/')
        .map_or(ark.len(), |i| LABEL.len() + i)
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}
