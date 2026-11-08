//! `SupportedApi::with_lenient` and the lenient parsers. Default behavior is
//! strict and unchanged.

use libtea_core::version::{OrderedPriority, VersionedEndpoint};
use libtea_core::{select_endpoints, ApiVersion, DiscoveryInfo, SupportedApi, WellKnownDocument};

fn well_known(versions: &str) -> Vec<u8> {
    format!(
        r#"{{"schemaVersion":1,"endpoints":[{{"url":"https://api.example.com","versions":{versions}}}]}}"#
    )
    .into_bytes()
}

#[test]
fn cybeats_shaped_short_version_is_rejected_by_default() {
    let json = well_known(r#"["1"]"#);
    assert!(WellKnownDocument::parse(&json).is_err());
    assert!(ApiVersion::parse("1").is_err());
    assert!(ApiVersion::parse("1.0").is_err());
}

#[test]
fn short_versions_parse_leniently_and_keep_their_spelling() {
    let json = well_known(r#"["1"]"#);
    let document = WellKnownDocument::parse_lenient(&json).unwrap();
    // Strict support does not match even after a lenient parse: 1 reads as 1.0.0
    // but is only selected when the support set is lenient too.
    let supported = SupportedApi::tea_1_0_0();
    let selected = document.select(&supported).unwrap();
    assert_eq!(
        selected[0].version, "1",
        "the path uses the advertised spelling"
    );
    let one = ApiVersion::parse_lenient("1").unwrap();
    assert_eq!(one.semver().to_string(), "1.0.0");
    assert_eq!(one.as_str(), "1");
    let one_two = ApiVersion::parse_lenient("1.2").unwrap();
    assert_eq!(one_two.semver().to_string(), "1.2.0");
}

#[test]
fn lenient_parse_still_rejects_nonsense() {
    for bad in [
        "", "v1", "01", "1.", ".1", "1.2.", "1.x", "1.0.0.0", "1..0", "-1", " 1",
    ] {
        assert!(ApiVersion::parse_lenient(bad).is_err(), "{bad:?}");
    }
    // Strict forms keep their strict meaning.
    assert_eq!(
        ApiVersion::parse_lenient("1.2.3-rc.1+build")
            .unwrap()
            .as_str(),
        "1.2.3-rc.1+build"
    );
}

fn endpoint(versions: &[&str]) -> VersionedEndpoint {
    VersionedEndpoint::new(
        "https://api.example.com",
        versions
            .iter()
            .map(|v| ApiVersion::parse_lenient(v).unwrap())
            .collect(),
        OrderedPriority::parse(1.0).unwrap(),
    )
}

#[test]
fn same_major_is_selected_only_when_lenient() {
    for advertised in ["1.0.1", "1.1.0", "1.9.9", "1", "1.1"] {
        let endpoints = [endpoint(&[advertised])];
        let strict = select_endpoints(&endpoints, &SupportedApi::tea_1_0_0());
        let lenient = select_endpoints(&endpoints, &SupportedApi::tea_1_0_0().with_lenient(true));
        if advertised == "1" {
            // `1` reads as 1.0.0, which an exact match accepts.
            assert!(strict.is_ok(), "{advertised}");
        } else {
            assert!(strict.is_err(), "strict must reject {advertised}");
        }
        assert_eq!(lenient.unwrap()[0].version, advertised);
    }
}

#[test]
fn other_majors_are_never_selected() {
    for advertised in ["2.0.0", "0.9.0", "2", "0.1"] {
        let endpoints = [endpoint(&[advertised])];
        for supported in [
            SupportedApi::tea_1_0_0(),
            SupportedApi::tea_1_0_0().with_lenient(true),
        ] {
            assert!(
                select_endpoints(&endpoints, &supported).is_err(),
                "{advertised}"
            );
        }
    }
    // Major 0 is exact-only even for a client that supports 0.x.
    let zero = SupportedApi::from_versions(["0.3.0"])
        .unwrap()
        .with_lenient(true);
    assert!(select_endpoints(&[endpoint(&["0.4.0"])], &zero).is_err());
    assert!(select_endpoints(&[endpoint(&["0.3.0"])], &zero).is_ok());
}

fn priority_endpoint(versions: &[&str], priority: f64) -> VersionedEndpoint {
    VersionedEndpoint::new(
        "https://api.example.com",
        versions
            .iter()
            .map(|v| ApiVersion::parse_lenient(v).unwrap())
            .collect(),
        OrderedPriority::parse(priority).unwrap(),
    )
}

fn order(endpoints: &[VersionedEndpoint], supported: &SupportedApi) -> Vec<String> {
    select_endpoints(endpoints, supported)
        .unwrap()
        .into_iter()
        .map(|selected| format!("{}@{}", selected.version, selected.priority.get()))
        .collect()
}

#[test]
fn exact_version_beats_a_higher_prerelease_even_at_lower_priority() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    // 1.99.0-rc.1 has priority 0 and is a pre-release: it is not selected at all.
    let endpoints = [
        priority_endpoint(&["1.0.0"], 1.0),
        priority_endpoint(&["1.99.0-rc.1"], 0.0),
    ];
    assert_eq!(order(&endpoints, &lenient), ["1.0.0@1"]);
}

#[test]
fn exact_beats_a_higher_stable_same_major_version_regardless_of_priority() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    let endpoints = [
        priority_endpoint(&["1.99.0"], 1.0),
        priority_endpoint(&["1.0.0"], 0.1),
        priority_endpoint(&["1.5.0"], 0.9),
    ];
    // Exact first, then same-major stable, highest version first.
    assert_eq!(
        order(&endpoints, &lenient),
        ["1.0.0@0.1", "1.99.0@1", "1.5.0@0.9"]
    );
}

#[test]
fn within_one_endpoint_exact_beats_same_major() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    let endpoints = [priority_endpoint(&["1.9.0", "1.0.0", "1.4.0"], 1.0)];
    assert_eq!(order(&endpoints, &lenient), ["1.0.0@1"]);
}

#[test]
fn exact_matches_are_ordered_by_priority() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    let endpoints = [
        priority_endpoint(&["1"], 0.2),
        priority_endpoint(&["1.0.0"], 0.8),
        priority_endpoint(&["1.0"], 0.5),
    ];
    assert_eq!(
        order(&endpoints, &lenient),
        ["1.0.0@0.8", "1.0@0.5", "1@0.2"]
    );
}

#[test]
fn prereleases_are_rejected_unless_they_match_exactly() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    for advertised in ["1.0.0-rc.1", "1.1.0-beta", "1.99.0-rc.1"] {
        let result = select_endpoints(&[priority_endpoint(&[advertised], 1.0)], &lenient);
        assert!(result.is_err(), "{advertised} must not be selected");
        assert!(!lenient.supports(ApiVersion::parse(advertised).unwrap().semver()));
    }
    // An exact pre-release support entry still matches itself.
    let beta = SupportedApi::from_versions(["1.1.0-beta"])
        .unwrap()
        .with_lenient(true);
    assert!(select_endpoints(&[priority_endpoint(&["1.1.0-beta"], 1.0)], &beta).is_ok());
    assert!(select_endpoints(&[priority_endpoint(&["1.2.0-beta"], 1.0)], &beta).is_err());
}

#[test]
fn short_version_one_is_still_accepted_as_an_exact_match() {
    let lenient = SupportedApi::tea_1_0_0().with_lenient(true);
    let selected = select_endpoints(&[priority_endpoint(&["1"], 1.0)], &lenient).unwrap();
    assert_eq!(selected[0].version, "1");
}

#[test]
fn strict_mode_ordering_is_unchanged() {
    let strict = SupportedApi::from_versions(["1.0.0", "1.1.0"]).unwrap();
    let endpoints = [
        priority_endpoint(&["1.0.0"], 1.0),
        priority_endpoint(&["1.1.0"], 0.1),
    ];
    // Highest version first, then priority (the pre-existing rule).
    assert_eq!(order(&endpoints, &strict), ["1.1.0@0.1", "1.0.0@1"]);
}

#[test]
fn discovery_lists_parse_leniently_on_request() {
    let json = br#"[{"productReleaseUuid":"d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
        "servers":[{"rootUrl":"https://api.example.com","versions":["1"]}]}]"#;
    assert!(DiscoveryInfo::parse_list(json).is_err());
    assert!(DiscoveryInfo::parse_list_lenient(json).is_ok());
}

#[test]
fn default_support_is_strict() {
    assert!(!SupportedApi::default().is_lenient());
    assert!(!SupportedApi::tea_1_0_0().is_lenient());
    assert!(!SupportedApi::from_versions(["1.0.0"]).unwrap().is_lenient());
}
