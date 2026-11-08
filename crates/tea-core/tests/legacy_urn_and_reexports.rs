//! `urn:tei:` gets its own error; third-party types are re-exported.

use libtea_core::url::Url;
use libtea_core::{DestinationPolicy, Tei, TeiError};

#[test]
fn legacy_urn_form_has_a_distinct_error() {
    for input in [
        "urn:tei:uuid:example.com:d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
        "URN:TEI:uuid:example.com:x",
        "Urn:Tei:",
    ] {
        let error = Tei::parse(input).unwrap_err();
        assert_eq!(error, TeiError::LegacyUrn, "{input}");
        let message = error.to_string();
        assert!(
            message.contains("legacy `urn:tei:` form is not supported"),
            "{message}"
        );
        assert!(message.contains("tei://"), "{message}");
    }
}

#[test]
fn other_bad_schemes_keep_the_scheme_error() {
    for input in [
        "urn:other:x",
        "tei:/example.com/uuid/x",
        "TEI://example.com/uuid/x",
        "urn:te",
        "ürn:tei:",
    ] {
        assert_eq!(Tei::parse(input).unwrap_err(), TeiError::Scheme, "{input}");
    }
}

#[test]
fn reexported_url_is_the_type_the_api_takes() {
    let origin = Url::parse("http://127.0.0.1:8080").unwrap();
    let policy = DestinationPolicy::allow_origin(&origin).unwrap();
    assert!(policy.allows_origin(&origin));
    let version: libtea_core::semver::Version = libtea_core::ApiVersion::parse("1.0.0")
        .unwrap()
        .semver()
        .clone();
    assert_eq!(version.major, 1);
    let _: &libtea_core::uuid::Uuid =
        libtea_core::TeaUuid::parse("d4d9f54a-abcf-11ee-ac79-1a52914d44b1")
            .unwrap()
            .as_uuid();
}
