//! Properties of `libtea_core::purl` that hold for any input it accepts.

use libtea_core::purl::{same_package_url, Purl, MAX_PURL_BYTES};
use libtea_core::{Identifier, Tei};
use proptest::prelude::*;

/// Strings shaped like PURLs, with separators, escapes and non-ASCII text.
fn purl_like() -> impl Strategy<Value = String> {
    let part = "[a-zA-Z0-9._~:+ @/%-]{0,12}|%[0-9A-Fa-f]{2}|[\u{e9}\u{20ac}]";
    (
        prop::sample::select(vec!["pkg:", "PKG:", "pkg://", "pkg:/"]),
        "[a-zA-Z][a-zA-Z0-9.-]{0,8}",
        prop::collection::vec(part, 0..4),
        "[a-zA-Z0-9._%-]{1,10}",
        prop::option::of("[a-zA-Z0-9.+_%-]{0,8}"),
        prop::collection::vec(
            ("[a-zA-Z][a-zA-Z0-9._-]{0,6}", "[a-zA-Z0-9.:/%,_-]{0,8}"),
            0..3,
        ),
        prop::option::of("[a-zA-Z0-9./%_-]{0,12}"),
    )
        .prop_map(|(scheme, ty, ns, name, version, qualifiers, subpath)| {
            let mut text = format!("{scheme}{ty}/");
            for segment in ns {
                text.push_str(&segment);
                text.push('/');
            }
            text.push_str(&name);
            if let Some(version) = version {
                text.push('@');
                text.push_str(&version);
            }
            for (index, (key, value)) in qualifiers.iter().enumerate() {
                text.push(if index == 0 { '?' } else { '&' });
                text.push_str(&format!("{key}={value}"));
            }
            if let Some(subpath) = subpath {
                text.push('#');
                text.push_str(&subpath);
            }
            text
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn canonical_is_idempotent_and_keeps_the_original(text in purl_like()) {
        if let Ok(purl) = Purl::parse(&text) {
            prop_assert_eq!(purl.as_str(), text.as_str());
            let canonical = purl.canonical();
            let reparsed = Purl::parse(canonical.as_str());
            prop_assert!(reparsed.is_ok(), "{} does not reparse", canonical);
            let reparsed = reparsed.unwrap();
            prop_assert_eq!(reparsed.canonical(), canonical.clone());
            prop_assert!(same_package_url(&purl, &reparsed));
            prop_assert!(same_package_url(&reparsed, &purl));
            prop_assert!(canonical.as_str().is_ascii());
        }
    }

    #[test]
    fn arbitrary_text_never_panics(text in ".{0,200}") {
        let _ = Purl::parse(&text).map(|purl| purl.canonical());
    }
}

#[test]
fn length_limit_is_inclusive() {
    let at_limit = format!(
        "pkg:generic/{}",
        "a".repeat(MAX_PURL_BYTES - "pkg:generic/".len())
    );
    assert_eq!(at_limit.len(), MAX_PURL_BYTES);
    assert!(Purl::parse(&at_limit).is_ok());
    assert!(Purl::parse(&format!("{at_limit}a")).is_err());
}

#[test]
fn a_purl_tei_is_used_as_received() {
    // pkg:pypi/Django@1.11.1 in unpadded Base64URL.
    let text = "tei://example.com/purl/cGtnOnB5cGkvRGphbmdvQDEuMTEuMQ";
    let tei = Tei::parse(text).unwrap();
    let purl = tei.purl().unwrap().unwrap();
    assert_eq!(purl.as_str(), "pkg:pypi/Django@1.11.1");
    assert_eq!(tei.to_string(), text);
    let bad = Tei::parse("tei://example.com/purl/@@@").unwrap();
    assert!(bad.purl().unwrap().is_err());
}

#[test]
fn identifier_purl_only_for_purl_identifiers() {
    let purl: Identifier =
        serde_json::from_str(r#"{"idType":"PURL","idValue":"PKG:npm/a@1"}"#).unwrap();
    let parsed = purl.purl().unwrap().unwrap();
    assert_eq!(parsed.canonical().as_str(), "pkg:npm/a@1");
    assert_eq!(purl.id_value, "PKG:npm/a@1");
    let cpe: Identifier =
        serde_json::from_str(r#"{"idType":"CPE","idValue":"cpe:2.3:a:x:y:1"}"#).unwrap();
    assert!(cpe.purl().is_none());
}
