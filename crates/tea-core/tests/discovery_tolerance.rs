//! Discovery documents with fields this crate does not know are accepted.
//!
//! The pinned schemas say `additionalProperties: false`, but a client that
//! rejects a server for adding a field cannot follow additive spec changes.
//! Strict-schema checking is a warning-level lint in `libtea-conformance`.

use libtea_core::{DiscoveryInfo, SupportedApi, WellKnownDocument};

#[test]
fn well_known_accepts_unknown_fields_at_every_level() {
    let json = br#"{
        "schemaVersion": 1,
        "futureTopLevel": {"a": 1},
        "endpoints": [{
            "url": "https://api.example.com",
            "versions": ["1.0.0"],
            "priority": 1,
            "region": "eu"
        }]
    }"#;
    let document = WellKnownDocument::parse(json).unwrap();
    let selected = document.select(&SupportedApi::tea_1_0_0()).unwrap();
    assert_eq!(selected[0].root_url, "https://api.example.com");
}

#[test]
fn discovery_accepts_unknown_fields_at_every_level() {
    let json = br#"[{
        "productReleaseUuid": "d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
        "futureField": true,
        "servers": [{
            "rootUrl": "https://api.example.com",
            "versions": ["1.0.0"],
            "priority": 0.5,
            "note": "primary"
        }]
    }]"#;
    let releases = DiscoveryInfo::parse_list(json).unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].servers[0].root_url, "https://api.example.com");
}

#[test]
fn required_fields_are_still_required() {
    assert!(WellKnownDocument::parse(br#"{"schemaVersion":1}"#).is_err());
    assert!(WellKnownDocument::parse(br#"{"schemaVersion":2,"endpoints":[]}"#).is_err());
    assert!(DiscoveryInfo::parse_list(br#"[{"servers":[]}]"#).is_err());
}
