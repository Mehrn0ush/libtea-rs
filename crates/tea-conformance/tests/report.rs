//! The offline report through the public API: case identifiers stay unique and
//! documented, the JSON shape stays stable, and the strict lints never panic or
//! warn on shapes they do not lint.

use std::collections::BTreeSet;
use std::path::Path;

use libtea_conformance::{lint_discovery_strict, lint_well_known_strict, static_report, Report};

fn matrix() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/interoperability.md"),
    )
    .unwrap()
}

#[test]
fn every_case_id_is_unique_and_listed_in_the_requirements_matrix() {
    let report = static_report();
    let ids: Vec<&str> = report.cases.iter().map(|case| case.id.as_str()).collect();
    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len(), "duplicate case ids: {ids:?}");

    let matrix = matrix();
    for id in ids {
        assert!(
            matrix.contains(&format!("| {id} |")),
            "{id} is not a row of the requirements matrix in docs/interoperability.md"
        );
    }
}

#[test]
fn every_case_passes_and_explains_itself() {
    let report = static_report();
    assert!(!report.cases.is_empty());
    for case in &report.cases {
        assert!(case.passed, "{} failed: {}", case.id, case.detail);
        assert!(!case.detail.trim().is_empty(), "{} has no detail", case.id);
    }
    assert!(report.passed());
}

#[test]
fn the_json_shape_is_stable_and_round_trips() {
    let report = static_report();
    let json: serde_json::Value = serde_json::from_str(&report.to_json().unwrap()).unwrap();
    let keys: BTreeSet<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        BTreeSet::from([
            "spec_commit",
            "spec_version",
            "is_certification",
            "cases",
            "warnings"
        ])
    );
    let case_keys: BTreeSet<&str> = json["cases"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(case_keys, BTreeSet::from(["id", "passed", "detail"]));

    let back: Report = serde_json::from_value(json).unwrap();
    assert_eq!(back, report);
}

#[test]
fn a_report_without_warnings_still_parses() {
    let mut json: serde_json::Value =
        serde_json::from_str(&static_report().to_json().unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("warnings");
    let report: Report = serde_json::from_value(json).unwrap();
    assert!(report.warnings.is_empty());
}

#[test]
fn a_failed_case_in_the_json_makes_the_report_fail() {
    let mut json: serde_json::Value =
        serde_json::from_str(&static_report().to_json().unwrap()).unwrap();
    json["cases"][0]["passed"] = serde_json::Value::Bool(false);
    let report: Report = serde_json::from_value(json).unwrap();
    assert!(!report.passed());
}

#[test]
fn the_lints_ignore_shapes_they_do_not_lint() {
    // Non-object entries, missing arrays and wrong top-level types: no warnings
    // and no panic. Parsing reports these documents as invalid elsewhere.
    for bytes in [
        &br#"{"schemaVersion":1,"endpoints":"x"}"#[..],
        br#"{"schemaVersion":1,"endpoints":[1,"a",null,[]]}"#,
        br#"[]"#,
        br#"null"#,
    ] {
        assert!(
            lint_well_known_strict(bytes).is_empty(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
    for bytes in [
        &br#"[1,"a",null,[]]"#[..],
        br#"[{"productReleaseUuid":"x","servers":"y"}]"#,
        br#"[{"productReleaseUuid":"x","servers":[1,null]}]"#,
        br#"{"servers":[]}"#,
    ] {
        assert!(
            lint_discovery_strict(bytes).is_empty(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
}

#[test]
fn the_lints_report_every_unknown_field_in_document_order_of_objects() {
    let well_known = br#"{"b":1,"schemaVersion":1,"a":2,
        "endpoints":[{"url":"u","versions":[],"x":1},{"url":"u","versions":[],"y":2}]}"#;
    let found: Vec<(String, String)> = lint_well_known_strict(well_known)
        .into_iter()
        .map(|w| (w.path, w.field))
        .collect();
    // serde_json objects iterate in key order, so `a` precedes `b`.
    assert_eq!(
        found,
        [
            ("$".to_owned(), "a".to_owned()),
            ("$".to_owned(), "b".to_owned()),
            ("$.endpoints[0]".to_owned(), "x".to_owned()),
            ("$.endpoints[1]".to_owned(), "y".to_owned()),
        ]
    );
}
