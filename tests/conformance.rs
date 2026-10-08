//! Conformance table for draft-kunze-ark-43.
//!
//! Every rule the service implements has an ID in `RULES`, and every case names the rules it
//! exercises. `every_rule_has_a_case` fails when a rule has no case.

use std::collections::{HashMap, HashSet};

use ark_service::ark::{ParseError, Violation, parse_ark};
use ark_service::check_character::calculate_check_character;
use ark_service::config::AppState;
use ark_service::error::AppError;
use ark_service::resolution::resolve;
use ark_service::shoulder::Shoulder;

const NAAN: &str = "26341";
const TIME_ATLAS: &str = "https://ark.timeatlas.eu/{pid}";
const VALUE: &str = "https://example.org/${value}";
const CONTENT: &str = "https://example.org/c/${content}";
const PREFIX_SCHEME_VALUE: &str = "https://example.org/${prefix}/${scheme}/${value}";
const APPEND: &str = "https://example.org/";
const QUERY: &str = "https://example.org/resolve?id={pid}";
const SUFFIX: &str = "https://example.org/${value}/manifest";
const LONG_NAME: &str = concat!(
    "ark:12345/x6",
    "bcdfghjkmnpqrstvwxz0123456789bcdfghjkmnpqrstvwxz0123456789",
    "bcdfghjkmnpqrstvwxz0123456789bcdfghjkmnpqrstvwxz0123456789",
    "bcdfghjkmnpqrstvwxz0123456789bcdfghjkmnpqrstvwxz0123456789",
    "bcdfghjkmnpqrstvwxz0123456789bcdfghjkmnpqrstvwxz0123456789",
    "bcdfghjkmnpqrstvwxz0123456789bcdfghjkmnpqrstvwxz0123456789",
);

const RULES: &[(&str, &str)] = &[
    (
        "2",
        "A check character covers the NAAN, '/', shoulder and blade",
    ),
    ("2.1", "The NMA is identity inert"),
    ("2.2", "The labels \"ark:/\" and \"ark:\" are equivalent"),
    (
        "2.3",
        "The NAAN is required and consists of betanumeric characters",
    ),
    ("2.3-length", "NAANs of 16 octets are supported"),
    ("2.4", "The Name is required"),
    (
        "2.4.1",
        "A primordinal shoulder is one or more betanumeric characters ending in a digit",
    ),
    (
        "2.5",
        "The Qualifier is a ComponentPath ('/'), a VariantPath ('.') or both",
    ),
    (
        "3.1-repertoire",
        "Name and Qualifier use letters, digits, = ~ * + @ _ $ and % - . /",
    ),
    ("3.1-percent", "'%' introduces two hex digits"),
    (
        "3.1-length",
        "A Base Name plus Qualifier of 255 octets is supported",
    ),
    ("3.1-hyphens", "Hyphens are identity inert"),
    (
        "3.1-transcription",
        "Whitespace is removed and U+2010 to U+2015 become hyphens",
    ),
    ("3.2-step1", "The NMA is removed"),
    ("3.2-step2", "The query string is removed"),
    (
        "3.2-step3",
        "The first case-insensitive \"ark:/\" or \"ark:\" becomes \"ark:\"",
    ),
    ("3.2-step4", "The NAAN is lowercased"),
    (
        "3.2-step5",
        "The two characters after every '%' are uppercased",
    ),
    ("3.2-step6", "All hyphens are removed"),
    (
        "3.2-step7",
        "No inflections are removed; they reach the target in the query string",
    ),
    (
        "3.2-step8",
        "Initial and final '/' and '.' are removed, runs collapse to their first",
    ),
    (
        "3.2-case",
        "Comparison is case-sensitive outside the label and NAAN",
    ),
    (
        "ncda",
        "The check character follows the Noid Check Digit Algorithm",
    ),
    (
        "resolve-match",
        "Resolution selects the shoulder by the normalized NAAN and shoulder",
    ),
    (
        "resolve-forward",
        "Template variables carry the parts of the ARK as received; its query joins the target query",
    ),
    (
        "resolve-label",
        "The forwarded ARK starts with the label \"ark:\"",
    ),
    (
        "resolve-conformant",
        "Only ARKs that conform to the specification are resolved",
    ),
    (
        "resolve-dot-segments",
        "A target path with a '.' or '..' segment, encoded or not, is not redirected to",
    ),
];

enum Expect {
    Normalized(&'static str),
    Equivalent(&'static str),
    NotEquivalent(&'static str),
    Parts {
        naan: &'static str,
        shoulder: Option<&'static str>,
        blade: &'static str,
        qualifier: &'static str,
    },
    NotAnArk(ParseError),
    Valid,
    Invalid(Violation),
    CheckCharacter(char),
    Redirect {
        pattern: &'static str,
        target: &'static str,
    },
    Unresolved(AppError),
}

struct Case {
    rules: &'static [&'static str],
    input: &'static str,
    expect: Expect,
}

use Expect::*;

const fn case(rules: &'static [&'static str], input: &'static str, expect: Expect) -> Case {
    Case {
        rules,
        input,
        expect,
    }
}

const fn redirect(
    rules: &'static [&'static str],
    pattern: &'static str,
    input: &'static str,
    target: &'static str,
) -> Case {
    case(rules, input, Redirect { pattern, target })
}

const CASES: &[Case] = &[
    case(
        &["2.1", "3.2-step1"],
        "http://example.org/rslvr/ark:12345/x6np1wh8k",
        Equivalent("ark:12345/x6np1wh8k"),
    ),
    case(
        &["2.1", "3.2-step1"],
        "https://example.com/ark:12345/x6np1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step1", "3.2-step3"],
        "https://n2t.net/ARK:/12345/x6np1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["2.2", "3.2-step3"],
        "ark:/12345/x6np1wh8k",
        Equivalent("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step3"],
        "ARK:12345/x6np1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step3"],
        "Ark:/12345/x6np1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step3"],
        "ark:12345/x6np1wh8k/ark:/z",
        Normalized("ark:12345/x6np1wh8k/ark:/z"),
    ),
    case(
        &["3.2-step4"],
        "ark:BCDFG/x6np1wh8k",
        Normalized("ark:bcdfg/x6np1wh8k"),
    ),
    case(&["2.3"], "ark:bcdfg/x6np1wh8k", Valid),
    case(
        &["2.3", "3.2-step4"],
        "ark:ABCDE/x6np1wh8k",
        Invalid(Violation::NaanNotBetanumeric),
    ),
    case(
        &["2.3"],
        "ark:12a45/x6np1wh8k",
        Invalid(Violation::NaanNotBetanumeric),
    ),
    case(&["2.3"], "ark:", NotAnArk(ParseError::MissingNaan)),
    case(&["2.4"], "ark:12345", NotAnArk(ParseError::MissingName)),
    case(
        &["2.4", "3.2-step8"],
        "ark:12345/",
        NotAnArk(ParseError::MissingName),
    ),
    case(
        &["2.4", "3.2-step8"],
        "ark://x6np1wh8k",
        NotAnArk(ParseError::MissingName),
    ),
    case(&["2.2"], "x6np1wh8k", NotAnArk(ParseError::MissingLabel)),
    case(
        &["2.1"],
        "https://example.org/x6np1wh8k",
        NotAnArk(ParseError::MissingLabel),
    ),
    case(
        &["2.4.1"],
        "ark:12345/x6np1wh8k/c2/s4.pdf",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "np1wh8k",
            qualifier: "/c2/s4.pdf",
        },
    ),
    case(
        &["2.4.1", "2.5"],
        "ark:12345/x6/np1wh8k/c2/s4.pdf",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "",
            qualifier: "/np1wh8k/c2/s4.pdf",
        },
    ),
    case(
        &["2.4.1"],
        "ark:12345/8np1wh8k",
        Parts {
            naan: "12345",
            shoulder: Some("8"),
            blade: "np1wh8k",
            qualifier: "",
        },
    ),
    case(
        &["2.4.1"],
        "ark:12345/abc/x6",
        Parts {
            naan: "12345",
            shoulder: None,
            blade: "abc",
            qualifier: "/x6",
        },
    ),
    case(
        &["2.4.1"],
        "ark:12345/bcd",
        Parts {
            naan: "12345",
            shoulder: None,
            blade: "bcd",
            qualifier: "",
        },
    ),
    case(
        &["2.4.1", "3.1-hyphens"],
        "ark:12345/x-6np1wh8k",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "np1wh8k",
            qualifier: "",
        },
    ),
    case(
        &["2.4.1", "3.1-hyphens"],
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000",
        Parts {
            naan: "26341",
            shoulder: Some("b1"),
            blade: "550e8400e29b41d4a716446655440000",
            qualifier: "",
        },
    ),
    case(
        &["2.5", "3.2-step1"],
        "https://example.org/ark:12345/x6np1wh8k/c3/s5.v7.xsl",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "np1wh8k",
            qualifier: "/c3/s5.v7.xsl",
        },
    ),
    case(
        &["2.5"],
        "ark:12345/x54.v18.fr.odf",
        Parts {
            naan: "12345",
            shoulder: Some("x5"),
            blade: "4",
            qualifier: ".v18.fr.odf",
        },
    ),
    case(
        &["2.5", "3.2-step2"],
        "ark:12345/x6np1wh8f?next=/a",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "np1wh8f",
            qualifier: "",
        },
    ),
    case(&["2.5"], "ark:12345/x6np1wh8f.pdf", Valid),
    case(&["3.1-repertoire"], "ark:12345/x6np1wh8k", Valid),
    case(&["3.1-repertoire"], "ark:12345/x6=~*+@_$", Valid),
    case(
        &["3.1-repertoire"],
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000",
        Valid,
    ),
    case(
        &["3.1-repertoire"],
        "ark:12345/x6np,1wh8k",
        Invalid(Violation::CharacterOutsideRepertoire),
    ),
    case(
        &["3.1-repertoire"],
        "ark:12345/x6np:1wh8k",
        Invalid(Violation::CharacterOutsideRepertoire),
    ),
    case(
        &["3.1-repertoire"],
        "ark:12345/x6np\u{f1}",
        Invalid(Violation::CharacterOutsideRepertoire),
    ),
    case(
        &["3.1-repertoire", "3.2-step2"],
        "ark:12345/x6np1wh8k?a=b,c",
        Valid,
    ),
    case(&["3.1-percent"], "ark:12345/x6np%7D", Valid),
    case(
        &["3.1-percent"],
        "ark:12345/x6np%zz",
        Invalid(Violation::MalformedPercentEncoding),
    ),
    case(
        &["3.1-percent"],
        "ark:12345/x6np%2",
        Invalid(Violation::MalformedPercentEncoding),
    ),
    case(
        &["3.1-hyphens", "3.2-step6"],
        "ark:12345/x5-4-xz-321",
        Equivalent("ark:12345/x54xz321"),
    ),
    case(
        &["3.1-hyphens", "3.2-step1", "3.2-step6"],
        "https://sneezy.dopey.com/ark:12345/x54--xz32-1",
        Equivalent("ark:12345/x54xz321"),
    ),
    case(
        &["3.1-transcription"],
        "ark:12345/x6np\n1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.1-transcription"],
        "ark:12345/x6np 1wh8k/\tpage2.pdf",
        Normalized("ark:12345/x6np1wh8k/page2.pdf"),
    ),
    case(
        &["3.1-transcription", "3.2-step6"],
        "ark:12345/x6np\u{2013}1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step2"],
        "ark:12345/x6np1wh8k?info",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step2"],
        "ark:12345/x6np1wh8k/c3?u=ark:/99999/z",
        Normalized("ark:12345/x6np1wh8k/c3"),
    ),
    case(
        &["3.2-step5"],
        "ark:12345/x6np%2fwh8k",
        Equivalent("ark:12345/x6np%2Fwh8k"),
    ),
    case(
        &["3.2-step5"],
        "ark:12345/x6%acT",
        Normalized("ark:12345/x6%ACT"),
    ),
    case(
        &["3.2-step5", "3.2-step6"],
        "ark:12345/x6%-2f",
        Normalized("ark:12345/x6%2f"),
    ),
    case(
        &["3.2-step7"],
        "ark:12345/x6np1wh8k??",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step8"],
        "ark:12345/x6np1wh8k//page2",
        Equivalent("ark:12345/x6np1wh8k/page2"),
    ),
    case(
        &["3.2-step8"],
        "ark:12345/x6np1wh8k./",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step8"],
        "ark:12345/x6np1wh8k..",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-step8"],
        "ark:12345/x6np1wh8k/c3/./s5",
        Normalized("ark:12345/x6np1wh8k/c3/s5"),
    ),
    case(
        &["3.2-step3", "3.2-step8"],
        "ark://12345/x6np1wh8k",
        Normalized("ark:12345/x6np1wh8k"),
    ),
    case(
        &["3.2-case"],
        "ark:12345/x6np1wh8k",
        NotEquivalent("ark:12345/X6NP1WH8K"),
    ),
    case(
        &["3.2-case"],
        "ark:12345/X6NP1WH8K",
        Normalized("ark:12345/X6NP1WH8K"),
    ),
    case(&["2", "ncda"], "ark:13030/xf93gt2", CheckCharacter('q')),
    case(
        &["2", "3.1-hyphens"],
        "ark:13030/xf93-gt2",
        CheckCharacter('q'),
    ),
    redirect(
        &["resolve-forward"],
        TIME_ATLAS,
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000",
        "https://ark.timeatlas.eu/ark:26341/b1550e8400-e29b-41d4-a716-446655440000",
    ),
    redirect(
        &["resolve-forward", "2.5"],
        TIME_ATLAS,
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000/page2.pdf",
        "https://ark.timeatlas.eu/ark:26341/b1550e8400-e29b-41d4-a716-446655440000/page2.pdf",
    ),
    redirect(
        &["resolve-forward", "2.5"],
        TIME_ATLAS,
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000.v2",
        "https://ark.timeatlas.eu/ark:26341/b1550e8400-e29b-41d4-a716-446655440000.v2",
    ),
    redirect(
        &["resolve-forward", "3.2-step7"],
        TIME_ATLAS,
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000?info",
        "https://ark.timeatlas.eu/ark:26341/b1550e8400-e29b-41d4-a716-446655440000?info",
    ),
    redirect(
        &["resolve-label", "2.2"],
        TIME_ATLAS,
        "ark:/26341/b1np1wh8k",
        "https://ark.timeatlas.eu/ark:26341/b1np1wh8k",
    ),
    redirect(
        &["resolve-label", "resolve-forward", "2.2"],
        CONTENT,
        "ark:/26341/b1np1wh8k",
        "https://example.org/c/26341/b1np1wh8k",
    ),
    redirect(
        &["resolve-forward", "3.2-step2"],
        TIME_ATLAS,
        "ark:26341/b1np1wh8k?u=ark:/99999/z",
        "https://ark.timeatlas.eu/ark:26341/b1np1wh8k?u=ark:/99999/z",
    ),
    redirect(
        &["resolve-match", "resolve-label", "3.2-step3"],
        TIME_ATLAS,
        "ARK:26341/b1np1wh8k",
        "https://ark.timeatlas.eu/ark:26341/b1np1wh8k",
    ),
    redirect(
        &["resolve-match", "resolve-forward", "3.1-hyphens"],
        TIME_ATLAS,
        "ark:26341/b-1np1wh8k",
        "https://ark.timeatlas.eu/ark:26341/b-1np1wh8k",
    ),
    redirect(
        &["resolve-match", "resolve-forward", "3.1-hyphens"],
        PREFIX_SCHEME_VALUE,
        "ark:263-41/b1np1wh8k",
        "https://example.org/263-41/ark/b1np1wh8k",
    ),
    redirect(
        &["resolve-forward", "2.5"],
        PREFIX_SCHEME_VALUE,
        "ark:26341/b1np1wh8k/c3/s5.v7.xsl",
        "https://example.org/26341/ark/b1np1wh8k/c3/s5.v7.xsl",
    ),
    redirect(
        &["resolve-forward", "3.2-step8"],
        VALUE,
        "ark:26341/b1np1wh8k/",
        "https://example.org/b1np1wh8k/",
    ),
    redirect(
        &["resolve-forward"],
        VALUE,
        "ark:26341/b1np1wh8k?next=/a",
        "https://example.org/b1np1wh8k?next=/a",
    ),
    redirect(
        &["resolve-forward", "3.2-step5"],
        VALUE,
        "ark:26341/b1np%2fwh8k",
        "https://example.org/b1np%2fwh8k",
    ),
    case(
        &["resolve-conformant"],
        "ark:26341/b1np{value}",
        Unresolved(AppError::InvalidArk),
    ),
    redirect(
        &["resolve-forward", "3.2-step7"],
        CONTENT,
        "ark:26341/b1550e8400-e29b-41d4-a716-446655440000?info",
        "https://example.org/c/26341/b1550e8400-e29b-41d4-a716-446655440000?info",
    ),
    redirect(
        &["resolve-forward", "2.5"],
        CONTENT,
        "ark:26341/b1np1wh8k/c3/s5.v7.xsl",
        "https://example.org/c/26341/b1np1wh8k/c3/s5.v7.xsl",
    ),
    redirect(
        &["resolve-forward", "3.2-step7"],
        APPEND,
        "ark:26341/b1np1wh8k??",
        "https://example.org/ark:26341/b1np1wh8k??",
    ),
    case(
        &["resolve-match"],
        "ark:12345/b1np1wh8k",
        Unresolved(AppError::InvalidNaan),
    ),
    case(
        &["resolve-match"],
        "ark:26341/x6np1wh8k",
        Unresolved(AppError::ShoulderNotFound),
    ),
    case(
        &["resolve-match", "2.4.1"],
        "ark:26341/bcd",
        Unresolved(AppError::ShoulderNotFound),
    ),
    case(
        &["resolve-match", "2.4"],
        "ark:26341",
        Unresolved(AppError::InvalidArk),
    ),
    case(&["2.3-length"], "ark:bcdfghjkmnpqrstv/x6np1wh8k", Valid),
    case(&["3.1-length"], LONG_NAME, Valid),
    case(
        &["3.2-step3", "3.2-step4", "3.2-step8"],
        "ark://BCDFG/x6np1wh8k",
        Invalid(Violation::NaanNotBetanumeric),
    ),
    case(
        &["2.1", "3.2-step1"],
        "n2t.net/ark:/12345/x6np1wh8k?redirect=https://example.org/",
        Parts {
            naan: "12345",
            shoulder: Some("x6"),
            blade: "np1wh8k",
            qualifier: "",
        },
    ),
    case(
        &["2.1", "3.2-step1"],
        "http://ark:8080/ark:26341/b1np1wh8k",
        Parts {
            naan: "26341",
            shoulder: Some("b1"),
            blade: "np1wh8k",
            qualifier: "",
        },
    ),
    redirect(
        &["resolve-label", "resolve-forward", "2.2"],
        PREFIX_SCHEME_VALUE,
        "ark://26341/b1np1wh8k",
        "https://example.org/26341/ark/b1np1wh8k",
    ),
    redirect(
        &["resolve-label", "3.1-hyphens"],
        TIME_ATLAS,
        "ark:-/26341/b1np1wh8k",
        "https://ark.timeatlas.eu/ark:26341/b1np1wh8k",
    ),
    redirect(
        &["resolve-forward", "3.2-step7"],
        QUERY,
        "ark:26341/b1np1wh8k?info",
        "https://example.org/resolve?id=ark:26341/b1np1wh8k&info",
    ),
    case(
        &["resolve-conformant", "3.1-repertoire"],
        "ark:26341/b1np1wh8k&x=1",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["resolve-conformant", "resolve-dot-segments"],
        "ark:26341/b1np1wh8k/..;/admin",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["resolve-conformant", "3.1-percent"],
        "ark:26341/b1np%zz",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["2.1", "3.2-step1"],
        "https://example.org/search?q=/ark:12345/x6np1wh8k",
        NotAnArk(ParseError::MissingLabel),
    ),
    redirect(
        &["resolve-forward", "3.2-step7"],
        SUFFIX,
        "ark:26341/b1np1wh8k?info",
        "https://example.org/b1np1wh8k/manifest?info",
    ),
    case(
        &["resolve-dot-segments"],
        "ark:26341/b1np1wh8k%2F..%2Fadmin",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["resolve-dot-segments"],
        "ark:26341/b1np1wh8k/../admin",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["resolve-dot-segments"],
        "ark:26341/b1np1wh8k/%2E%2e/admin",
        Unresolved(AppError::InvalidArk),
    ),
    case(
        &["resolve-dot-segments"],
        "ark:26341/b1np1wh8k\\..\\admin",
        Unresolved(AppError::InvalidArk),
    ),
];

fn state(pattern: &str) -> AppState {
    let shoulder = Shoulder {
        route_pattern: pattern.to_string(),
        project_name: "Test".to_string(),
        ..Default::default()
    };
    AppState {
        naan: NAAN.to_string(),
        default_blade_length: 8,
        max_mint_count: 1000,
        shoulders: HashMap::from([("b1".to_string(), shoulder)]),
    }
}

fn check(case: &Case) -> Result<(), String> {
    let input = case.input;
    match &case.expect {
        Normalized(expected) => {
            let actual = parse_ark(input).map(|ark| ark.normalized().to_string());
            if actual.as_deref() != Ok(*expected) {
                return Err(format!("normalized to {actual:?}, expected {expected:?}"));
            }
        }
        Equivalent(other) | NotEquivalent(other) => {
            let equal = parse_ark(input).map_err(|e| e.to_string())?
                == parse_ark(other).map_err(|e| e.to_string())?;
            if equal != matches!(case.expect, Equivalent(_)) {
                return Err(format!("equivalence with {other:?} is {equal}"));
            }
        }
        Parts {
            naan,
            shoulder,
            blade,
            qualifier,
        } => {
            let ark = parse_ark(input).map_err(|e| e.to_string())?;
            let actual = (ark.naan(), ark.shoulder(), ark.blade(), ark.qualifier());
            if actual != (*naan, *shoulder, *blade, *qualifier) {
                return Err(format!("parsed into {actual:?}"));
            }
        }
        NotAnArk(expected) => {
            let actual = parse_ark(input).err();
            if actual != Some(*expected) {
                return Err(format!("parse error {actual:?}, expected {expected:?}"));
            }
        }
        Valid | Invalid(_) => {
            let ark = parse_ark(input).map_err(|e| e.to_string())?;
            let expected = match case.expect {
                Invalid(v) => Some(v),
                _ => None,
            };
            if ark.violation() != expected {
                return Err(format!(
                    "violation {:?}, expected {expected:?}",
                    ark.violation()
                ));
            }
        }
        CheckCharacter(expected) => {
            let ark = parse_ark(input).map_err(|e| e.to_string())?;
            let actual = calculate_check_character(&ark.check_zone());
            if actual != *expected {
                return Err(format!("check character {actual:?}, expected {expected:?}"));
            }
        }
        Redirect { pattern, target } => {
            let actual = resolve(&state(pattern), input).map_err(|e| format!("{e:?}"))?;
            if actual != *target {
                return Err(format!("redirected to {actual:?}"));
            }
        }
        Unresolved(expected) => {
            let actual = resolve(&state(TIME_ATLAS), input);
            if actual != Err(*expected) {
                return Err(format!("resolved to {actual:?}"));
            }
        }
    }
    Ok(())
}

#[test]
fn every_case_passes() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|case| {
            check(case)
                .err()
                .map(|e| format!("{:?} {:?}: {e}", case.rules, case.input))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_rule_has_a_case() {
    let known: HashSet<&str> = RULES.iter().map(|(id, _)| *id).collect();
    let used: HashSet<&str> = CASES.iter().flat_map(|c| c.rules.iter().copied()).collect();

    let unknown: Vec<_> = used.difference(&known).collect();
    assert!(unknown.is_empty(), "cases cite unknown rules: {unknown:?}");

    let uncovered: Vec<_> = RULES
        .iter()
        .filter(|(id, _)| !used.contains(id))
        .map(|(id, rule)| format!("{id}: {rule}"))
        .collect();
    assert!(
        uncovered.is_empty(),
        "rules without a case:\n{}",
        uncovered.join("\n")
    );
}
