//! Checks derived from the pinned TEA specification.
//!
//! Passing these checks does not make a server conformant. A server is not
//! conformant merely because it interoperates with this SDK. The checks record
//! what this client requires of inputs it accepts.

#![forbid(unsafe_code)]

use libtea_core::{
    verify_checksums, Checksum, ChecksumAlgorithm, DiscoveryInfo, SupportedApi, Tei,
    WellKnownDocument,
};
use serde::{Deserialize, Serialize};

/// One specification-derived check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CaseResult {
    /// Stable identifier used in the requirements matrix.
    pub id: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Short explanation.
    pub detail: String,
}

/// A non-failing finding from a strict-schema lint.
///
/// The pinned schemas declare `additionalProperties: false` for the
/// well-known and discovery documents. `libtea-client` accepts extra fields so
/// that additive server changes do not break discovery; this lint reports
/// them. A warning never makes [`Report::passed`] false.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LintWarning {
    /// JSON path of the offending object, for example `$.endpoints[0]`.
    pub path: String,
    /// The field the pinned schema does not define.
    pub field: String,
    /// Short explanation.
    pub detail: String,
}

impl LintWarning {
    fn unknown(path: &str, field: &str) -> Self {
        Self {
            path: path.to_owned(),
            field: field.to_owned(),
            detail: "field is not defined by the pinned schema (additionalProperties: false)"
                .into(),
        }
    }
}

const WELL_KNOWN_KEYS: &[&str] = &["schemaVersion", "endpoints"];
const ENDPOINT_KEYS: &[&str] = &["url", "versions", "priority"];
const DISCOVERY_KEYS: &[&str] = &["productReleaseUuid", "servers"];
const SERVER_KEYS: &[&str] = &["rootUrl", "versions", "priority"];

fn lint_object(
    value: &serde_json::Value,
    path: &str,
    allowed: &[&str],
    out: &mut Vec<LintWarning>,
) {
    if let Some(object) = value.as_object() {
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                out.push(LintWarning::unknown(path, key));
            }
        }
    }
}

/// Warn about fields in a `.well-known/tea` document that the pinned schema
/// does not define. Invalid JSON yields no warnings (parsing reports that).
#[must_use]
pub fn lint_well_known_strict(bytes: &[u8]) -> Vec<LintWarning> {
    let mut out = Vec::new();
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return out;
    };
    lint_object(&document, "$", WELL_KNOWN_KEYS, &mut out);
    if let Some(endpoints) = document.get("endpoints").and_then(|v| v.as_array()) {
        for (index, endpoint) in endpoints.iter().enumerate() {
            lint_object(
                endpoint,
                &format!("$.endpoints[{index}]"),
                ENDPOINT_KEYS,
                &mut out,
            );
        }
    }
    out
}

/// Warn about fields in a `/discovery` response that the pinned schema does
/// not define. Invalid JSON yields no warnings.
#[must_use]
pub fn lint_discovery_strict(bytes: &[u8]) -> Vec<LintWarning> {
    let mut out = Vec::new();
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return out;
    };
    let Some(items) = document.as_array() else {
        return out;
    };
    for (index, item) in items.iter().enumerate() {
        let path = format!("$[{index}]");
        lint_object(item, &path, DISCOVERY_KEYS, &mut out);
        if let Some(servers) = item.get("servers").and_then(|v| v.as_array()) {
            for (server_index, server) in servers.iter().enumerate() {
                lint_object(
                    server,
                    &format!("{path}.servers[{server_index}]"),
                    SERVER_KEYS,
                    &mut out,
                );
            }
        }
    }
    out
}

/// A machine-readable offline smoke report.
///
/// Embeds the pinned OpenAPI commit. **Not** a CycloneDX / ECMA certification
/// and **not** evidence that any server is conformant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Report {
    /// Pinned specification commit (`libtea_core::spec::COMMIT`).
    pub spec_commit: String,
    /// OpenAPI `info.version`.
    pub spec_version: String,
    /// Always `false` for this crate: these checks are smoke, not certification.
    pub is_certification: bool,
    /// Individual checks.
    pub cases: Vec<CaseResult>,
    /// Lint findings. Never affects [`Report::passed`]. Empty for
    /// [`static_report`]; fill it with [`lint_well_known_strict`] and
    /// [`lint_discovery_strict`] output for a document under test.
    #[serde(default)]
    pub warnings: Vec<LintWarning>,
}

impl Report {
    /// Whether every case passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.cases.iter().all(|case| case.passed)
    }

    /// JSON encoding of the report.
    ///
    /// # Errors
    ///
    /// Returns an error if the report cannot be serialized.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Run the static checks that do not contact a network.
#[must_use]
pub fn static_report() -> Report {
    let cases = vec![
        tei_examples(),
        tei_equality(),
        well_known_selection(),
        empty_discovery(),
        unknown_error_is_still_a_tea_error(),
        checksum_does_not_claim_success_when_wrong(),
        weak_checksum_alone_is_not_verified(),
    ];
    Report {
        spec_commit: libtea_core::spec::COMMIT.to_owned(),
        spec_version: libtea_core::spec::SPEC_VERSION.to_owned(),
        is_certification: false,
        cases,
        warnings: Vec::new(),
    }
}

fn tei_examples() -> CaseResult {
    let samples = [
        "tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
        "tei://example.com/purl/cGtnOm1hdmVuL2NvbW1vbnMtaW8vY29tbW9ucy1pb0AyLjIyLjA",
        "tei://example.com/hash/SHA-256:fd44efd601f651c8865acf0dfeacb0df19a2b50ec69ead0262096fd2f67197b9",
        "tei://example.com/eanupc/1234567890128",
        "tei://example.com/gtin/0234567890129",
        "tei://example.com/asin/B07FZ8S74R",
        "tei://example.com/udi/00123456789012",
    ];
    let passed = samples.iter().all(|sample| {
        Tei::parse(sample)
            .and_then(|tei| tei.validate_registry().map(|()| tei))
            .is_ok_and(|tei| tei.as_str() == *sample)
    });
    CaseResult {
        id: "TEA-TEI-001".into(),
        passed,
        detail: "registry examples parse, validate, and keep their original spelling".into(),
    }
}

fn tei_equality() -> CaseResult {
    let left = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1");
    let right = Tei::parse("tei://Example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1");
    let passed = match (left, right) {
        (Ok(left), Ok(right)) => left != right,
        _ => false,
    };
    CaseResult {
        id: "TEA-TEI-002".into(),
        passed,
        detail: "TEI comparison is case-sensitive and does not rewrite the domain".into(),
    }
}

fn well_known_selection() -> CaseResult {
    let json = br#"{
      "schemaVersion": 1,
      "endpoints": [
        {"url": "https://api.example.com", "versions": ["1.0.0"], "priority": 1},
        {"url": "https://api2.example.com/mytea", "versions": ["1.0.0"], "priority": 0.5}
      ]
    }"#;
    let passed = WellKnownDocument::parse(json)
        .and_then(|doc| doc.select(&SupportedApi::tea_1_0_0()))
        .is_ok_and(|selected| {
            selected[0].root_url == "https://api.example.com" && selected[0].version == "1.0.0"
        });
    CaseResult {
        id: "TEA-DISC-001".into(),
        passed,
        detail: "highest equal version is ordered by priority, with the normative example".into(),
    }
}

fn empty_discovery() -> CaseResult {
    let passed = DiscoveryInfo::parse_list(b"[]").is_err();
    CaseResult {
        id: "TEA-DISC-002".into(),
        passed,
        detail: "an empty discovery array is not a successful resolution".into(),
    }
}

fn unknown_error_is_still_a_tea_error() -> CaseResult {
    let code = libtea_core::tea_error_code(
        Some("application/json; charset=utf-8"),
        br#"{"error":"FUTURE_CODE"}"#,
    );
    CaseResult {
        id: "TEA-DISC-003".into(),
        passed: code.as_deref() == Some("FUTURE_CODE"),
        detail:
            "an unknown error string still identifies a TEA error and is not a failover trigger"
                .into(),
    }
}

fn checksum_does_not_claim_success_when_wrong() -> CaseResult {
    let checksum = Checksum::new(
        ChecksumAlgorithm::Sha256,
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let passed = checksum.is_ok_and(|checksum| {
        !matches!(
            verify_checksums(b"tea", &[checksum]).outcome,
            libtea_core::IntegrityOutcome::Verified
        )
    });
    CaseResult {
        id: "TEA-ART-001".into(),
        passed,
        detail: "a mismatched checksum is not reported as verified".into(),
    }
}

fn weak_checksum_alone_is_not_verified() -> CaseResult {
    // SHA-1("tea")
    let checksum = Checksum::new(
        ChecksumAlgorithm::Sha1,
        "75490f32c06a50051804339453d3fe2f0cd5564b",
    );
    let passed = checksum.is_ok_and(|checksum| {
        let outcome = verify_checksums(b"tea", std::slice::from_ref(&checksum)).outcome;
        outcome == libtea_core::IntegrityOutcome::Weak
            && libtea_core::verify_checksums_with_policy(
                b"tea",
                &[checksum],
                libtea_core::ChecksumPolicy::new().with_allow_weak(true),
            )
            .outcome
                == libtea_core::IntegrityOutcome::Verified
    });
    CaseResult {
        id: "TEA-ART-002".into(),
        passed,
        detail: "a SHA-1-only match is Weak, not Verified, unless the caller opts in".into(),
    }
}

#[cfg(doctest)]
mod non_exhaustive_guards;

/// The workspace `README.md`, compiled as doctests so its Rust snippets cannot
/// drift from the API. This crate is unpublished, so the path outside the crate
/// directory does not affect packaging.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
pub struct RootReadmeDoctests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_suite_passes() {
        let report = static_report();
        assert!(report.passed(), "{}", report.to_json().unwrap());
    }

    #[test]
    fn strict_lint_warns_about_unknown_fields_and_never_fails_the_report() {
        let well_known = br#"{"schemaVersion":1,"extra":1,
            "endpoints":[{"url":"https://a.example","versions":["1.0.0"],"region":"eu"}]}"#;
        let warnings = lint_well_known_strict(well_known);
        let found: Vec<_> = warnings
            .iter()
            .map(|w| (w.path.as_str(), w.field.as_str()))
            .collect();
        assert_eq!(found, [("$", "extra"), ("$.endpoints[0]", "region")]);
        // The client accepts the same bytes.
        assert!(libtea_core::WellKnownDocument::parse(well_known).is_ok());

        let discovery = br#"[{"productReleaseUuid":"d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
            "note":"x","servers":[{"rootUrl":"https://a.example","versions":["1.0.0"],"zone":"z"}]}]"#;
        let found: Vec<_> = lint_discovery_strict(discovery)
            .into_iter()
            .map(|w| (w.path, w.field))
            .collect();
        assert_eq!(
            found,
            [
                ("$[0]".to_owned(), "note".to_owned()),
                ("$[0].servers[0]".to_owned(), "zone".to_owned())
            ]
        );
        assert!(libtea_core::DiscoveryInfo::parse_list(discovery).is_ok());

        let mut report = static_report();
        report.warnings = warnings;
        assert!(report.passed(), "warnings must not fail the report");
        let json = report.to_json().unwrap();
        assert!(json.contains("\"warnings\""));
    }

    #[test]
    fn strict_lint_is_quiet_for_valid_documents_and_bad_json() {
        let valid = br#"{"schemaVersion":1,"endpoints":[{"url":"https://a.example","versions":["1.0.0"],"priority":1}]}"#;
        assert!(lint_well_known_strict(valid).is_empty());
        assert!(lint_well_known_strict(b"not json").is_empty());
        assert!(lint_discovery_strict(b"{}").is_empty());
        assert!(lint_discovery_strict(b"[]").is_empty());
    }

    #[test]
    fn report_is_not_a_certification_and_names_the_pinned_commit() {
        let report = static_report();
        assert!(!report.is_certification);
        assert_eq!(
            report.spec_commit,
            "962a0228af1bfcc1232ac3f79239317ff33fdb04"
        );
        assert_eq!(report.spec_commit, libtea_core::spec::COMMIT);
        let json: serde_json::Value = serde_json::from_str(&report.to_json().unwrap()).unwrap();
        assert_eq!(json["is_certification"], serde_json::Value::Bool(false));
        assert_eq!(json["spec_commit"], libtea_core::spec::COMMIT);
    }
}
