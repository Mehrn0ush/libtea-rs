//! Checks derived from the pinned TEA specification.
//!
//! Passing these checks does not make a server conformant. A server is not
//! conformant merely because it interoperates with this SDK. The checks record
//! what this client requires of inputs it accepts.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use tea_core::{
    verify_checksums, Checksum, ChecksumAlgorithm, DiscoveryInfo, SupportedApi, Tei,
    WellKnownDocument,
};

/// One specification-derived check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseResult {
    /// Stable identifier used in the requirements matrix.
    pub id: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Short explanation.
    pub detail: String,
}

/// A machine-readable conformance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// Pinned specification commit.
    pub spec_commit: String,
    /// OpenAPI `info.version`.
    pub spec_version: String,
    /// Individual checks.
    pub cases: Vec<CaseResult>,
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
    ];
    Report {
        spec_commit: tea_core::spec::COMMIT.to_owned(),
        spec_version: tea_core::spec::SPEC_VERSION.to_owned(),
        cases,
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
    let code = tea_core::tea_error_code(
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
            tea_core::IntegrityOutcome::Verified
        )
    });
    CaseResult {
        id: "TEA-ART-001".into(),
        passed,
        detail: "a mismatched checksum is not reported as verified".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_suite_passes() {
        let report = static_report();
        assert!(report.passed(), "{}", report.to_json().unwrap());
    }
}
