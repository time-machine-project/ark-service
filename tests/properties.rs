use std::collections::HashMap;

use ark_service::ark::{BETANUMERIC, parse_ark};
use ark_service::check_character::{calculate_check_character, validate_check_character};
use ark_service::config::AppState;
use ark_service::minting::mint_ark;
use ark_service::shoulder::{Shoulder, parse_shoulders};
use ark_service::validation::validate_ark;
use proptest::prelude::*;
use proptest::sample::select;

fn normalized(ark: &str) -> Option<String> {
    parse_ark(ark).ok().map(|ark| ark.normalized().to_string())
}

/// A normalized ARK: betanumeric NAAN, a base Name and up to two qualifier segments.
fn canonical_ark() -> impl Strategy<Value = String> {
    (
        "[0-9bcdfghjkmnpqrstvwxz]{1,8}",
        "[0-9a-zA-Z=~*+@_$]{1,12}",
        prop::collection::vec("[/.][0-9a-zA-Z=~*+@_$]{1,6}", 0..3),
    )
        .prop_map(|(naan, base, qualifiers)| format!("ark:{naan}/{base}{}", qualifiers.concat()))
}

/// The same ARK as published or transcribed: NMA, label form, a separator after the label,
/// NAAN case, hyphens, doubled and trailing structural characters, whitespace and a query
/// string.
fn variant_of(canonical: String) -> impl Strategy<Value = (String, String)> {
    let content = canonical["ark:".len()..].to_string();
    let len = content.chars().count();
    (
        select(vec!["", "https://n2t.net/", "http://example.org/rslvr/"]),
        select(vec!["ark:", "ARK:", "Ark:", "ark:/", "ARK:/"]),
        select(vec!["", "-", "/"]),
        prop::collection::vec(any::<bool>(), len),
        prop::collection::vec(any::<bool>(), len),
        prop::collection::vec(any::<bool>(), len),
        select(vec!["", "/", ".", "./", "//"]),
        select(vec!["", "?info", "??", "?next=/a"]),
        prop::collection::vec((0..=len, select(vec![" ", "\n", "\t"])), 0..3),
    )
        .prop_map(
            move |(nma, label, separator, upper, hyphens, doubled, trailing, query, spaces)| {
                let naan_len = content.find('/').unwrap();
                // §3.2 step 4 runs before step 8 removes a '/' after the label, so an uppercase
                // NAAN there stays uppercase; the conformance table covers that case.
                let uppercase_naan = separator != "/";
                let mut body = String::from(separator);
                for (i, c) in content.chars().enumerate() {
                    if hyphens[i] {
                        body.push('-');
                    }
                    body.push(if i < naan_len && uppercase_naan && upper[i] {
                        c.to_ascii_uppercase()
                    } else {
                        c
                    });
                    if (c == '/' || c == '.') && doubled[i] {
                        body.push(if c == '/' { '.' } else { '/' });
                    }
                }
                let mut variant: Vec<char> = format!("{nma}{label}{body}{trailing}{query}")
                    .chars()
                    .collect();
                for (position, space) in &spaces {
                    let at =
                        (nma.len() + label.len() + separator.len() + position).min(variant.len());
                    variant.splice(at..at, space.chars());
                }
                (canonical.clone(), variant.into_iter().collect())
            },
        )
}

fn betanumeric_string(max_len: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(select(BETANUMERIC.to_vec()), 1..=max_len)
        .prop_map(|bytes| String::from_utf8(bytes).unwrap())
}

proptest! {
    #[test]
    fn canonical_arks_are_their_own_normal_form(ark in canonical_ark()) {
        prop_assert_eq!(normalized(&ark), Some(ark.clone()));
    }

    #[test]
    fn variants_normalize_to_the_canonical_ark(
        (canonical, variant) in canonical_ark().prop_flat_map(variant_of)
    ) {
        prop_assert_eq!(normalized(&variant), Some(canonical.clone()));
        let parsed = parse_ark(&variant).unwrap();
        prop_assert_eq!(normalized(parsed.normalized()), Some(canonical.clone()));
        prop_assert!(parsed == parse_ark(&canonical).unwrap());
    }

    #[test]
    fn letter_case_in_the_name_is_significant(ark in canonical_ark()) {
        let name_start = ark.find('/').unwrap() + 1;
        let flipped: String = ark[name_start..]
            .chars()
            .map(|c| if c.is_ascii_lowercase() { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() })
            .collect();
        prop_assume!(flipped != ark[name_start..]);
        let other = format!("{}{flipped}", &ark[..name_start]);
        prop_assert!(parse_ark(&ark).unwrap() != parse_ark(&other).unwrap());
    }

    #[test]
    fn received_parts_are_slices_of_the_received_ark(
        (_, variant) in canonical_ark().prop_flat_map(variant_of)
    ) {
        let ark = parse_ark(&variant).unwrap();
        prop_assert_eq!(format!("ark:{}", ark.content()), ark.pid());
        prop_assert_eq!(format!("{}/{}", ark.prefix(), ark.value()), ark.content());
        let query = ark.query().map(|q| format!("?{q}")).unwrap_or_default();
        prop_assert_eq!(format!("{}{query}", ark.pid()), ark.received());
        prop_assert!(variant.replace([' ', '\n', '\t'], "").ends_with(&ark.received()["ark:".len()..]));
    }

    #[test]
    fn minted_arks_validate(
        shoulder in "[bcdfghjkmnpqrstvwxz]{0,3}[0-9]",
        blade_length in 1usize..12,
        uses_check_character: bool,
    ) {
        let naan = "26341";
        let ark = mint_ark(naan, &shoulder, blade_length, uses_check_character);
        let state = AppState {
            naan: naan.to_string(),
            default_blade_length: blade_length,
            max_mint_count: 1,
            shoulders: HashMap::from([(shoulder.clone(), Shoulder::default())]),
        };

        let result = validate_ark(&state, &ark, Some(uses_check_character));
        prop_assert!(result.valid);
        prop_assert_eq!(result.shoulder.as_deref(), Some(shoulder.as_str()));
        prop_assert_eq!(result.naan_matches, Some(true));
        prop_assert_eq!(result.shoulder_registered, Some(true));
        prop_assert_eq!(result.check_character_valid, uses_check_character.then_some(true));
        prop_assert!(result.warnings.is_empty());
    }

    #[test]
    fn configured_shoulders_are_the_shoulders_of_their_minted_arks(key in "[a-z0-9]{1,4}") {
        let accepted = parse_shoulders(&format!("{key}\thttps://example.org/\tTest")).is_ok();
        let ark = mint_ark("26341", &key, 8, true);
        let parsed_back = parse_ark(&ark).unwrap().shoulder() == Some(key.as_str());
        prop_assert_eq!(accepted, parsed_back);
    }

    #[test]
    fn check_character_detects_substitutions_and_adjacent_transpositions(
        base in betanumeric_string(27)
    ) {
        let correct = format!("{base}{}", calculate_check_character(&base)).into_bytes();
        for position in 0..correct.len() {
            for &replacement in BETANUMERIC {
                if replacement != correct[position] {
                    let mut wrong = correct.clone();
                    wrong[position] = replacement;
                    prop_assert!(!validate_check_character(std::str::from_utf8(&wrong).unwrap()));
                }
            }
            if position + 1 < correct.len() && correct[position] != correct[position + 1] {
                let mut swapped = correct.clone();
                swapped.swap(position, position + 1);
                prop_assert!(!validate_check_character(std::str::from_utf8(&swapped).unwrap()));
            }
        }
    }
}
