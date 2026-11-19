//! The `vers` test suite (`spec/vers/`, vers-spec v1.2.1) against
//! `libtea_core::vers`.
//!
//! `parse` cases compare the type and constraints, and an expected failure
//! must be rejected by `VersRange::parse` or, for a SemVer type, by the
//! validation `contains_semver` runs first. `containment` cases are evaluated
//! for SemVer types. `validate` cases expect a normalized string, which this
//! module does not produce, so they are skipped. Cases that need rules for a
//! type this module does not evaluate are listed in `TYPE_SPECIFIC`; the test
//! fails if one of them starts passing.

use std::path::Path;

use libtea_core::vers::{Comparator, VersRange};
use serde_json::Value;

const TYPE_SPECIFIC: &[(&str, &str)] = &[
    (
        "vers:datetime/2024-01-01t00:00:00z",
        "datetime versions use uppercase T and Z",
    ),
    (
        "vers:datetime/2024-01-01T00%3A00%3A00Z",
        "datetime colons are not percent-encoded",
    ),
    (
        "vers:datetime/2024-01-01T00%3a00%3a00Z",
        "datetime colons are not percent-encoded",
    ),
];

fn cases(file: &str) -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vers/tests")
        .join(file);
    let data: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    data["tests"].as_array().unwrap().clone()
}

fn comparator(c: Comparator) -> &'static str {
    match c {
        Comparator::Equal => "=",
        other => other.as_str(),
    }
}

fn shape(range: &VersRange) -> Value {
    let constraints: Vec<Value> = range
        .constraints()
        .iter()
        .map(|c| serde_json::json!([comparator(c.comparator()), c.version()]))
        .collect();
    serde_json::json!({"type": range.version_type(), "constraints": constraints})
}

/// `Ok` with the parsed shape, or `Err` when rejected at parse or, for a
/// SemVer type, by the validation evaluation runs first.
fn parse_and_validate(input: &str) -> Result<Value, String> {
    let range = VersRange::parse(input).map_err(|err| err.to_string())?;
    if range.is_semver_type() {
        range
            .contains_semver(&semver::Version::new(0, 0, 0))
            .map_err(|err| err.to_string())?;
    }
    Ok(shape(&range))
}

#[test]
fn canonical_parse_cases() {
    let mut failures = Vec::new();
    let mut passed = 0;
    for case in cases("vers_canonical_parse_test.json") {
        if case["test_type"] != "parse" {
            continue;
        }
        let input = case["input"].as_str().unwrap();
        let fails = case["expected_failure"].as_bool().unwrap_or(false);
        // A success case is about syntax, so only `parse` is asked; its
        // versions need not be SemVer (`1.0%2F0`).
        let ok = if fails {
            parse_and_validate(input).is_err()
        } else {
            VersRange::parse(input).is_ok_and(|range| shape(&range) == case["expected_output"])
        };
        let listed = TYPE_SPECIFIC.iter().any(|(i, _)| *i == input);
        match (ok, listed) {
            (true, false) => passed += 1,
            (false, true) => {}
            (true, true) => {
                failures.push(format!("{input} now passes; remove it from TYPE_SPECIFIC"))
            }
            (false, false) => failures.push(format!("{input}: {:?}", parse_and_validate(input))),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(passed >= 15, "only {passed} cases ran");
}

#[test]
fn containment_cases() {
    let mut ran = 0;
    for case in cases("npm_range_containment_test.json") {
        let range = VersRange::parse(case["input"]["vers"].as_str().unwrap()).unwrap();
        let version = semver::Version::parse(case["input"]["version"].as_str().unwrap()).unwrap();
        assert_eq!(
            range.contains_semver(&version).unwrap(),
            case["expected_output"].as_bool().unwrap(),
            "{case}"
        );
        ran += 1;
    }
    assert!(ran >= 1);
}

#[test]
fn every_type_specific_entry_names_a_case() {
    let inputs: Vec<String> = cases("vers_canonical_parse_test.json")
        .iter()
        .filter_map(|case| case["input"].as_str().map(str::to_owned))
        .collect();
    for (input, reason) in TYPE_SPECIFIC {
        assert!(!reason.is_empty());
        assert!(
            inputs.iter().any(|i| i == input),
            "{input} is not in the suite"
        );
    }
}

#[test]
fn vendored_files_match_the_pinned_digests() {
    use sha2::{Digest, Sha256};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pinned = std::fs::read_to_string(root.join("spec/vers/PINNED.md")).unwrap();
    let mut checked = 0;
    for line in pinned
        .lines()
        .filter(|line| line.starts_with("| `spec/vers/"))
    {
        let cells: Vec<_> = line
            .split('|')
            .map(|cell| cell.trim().trim_matches('`'))
            .collect();
        let bytes = std::fs::read(root.join(cells[1])).unwrap();
        let actual: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, cells[3], "{}", cells[1]);
        checked += 1;
    }
    let on_disk = std::fs::read_dir(root.join("spec/vers/tests"))
        .unwrap()
        .count()
        + 1;
    assert_eq!(checked, on_disk);
}
