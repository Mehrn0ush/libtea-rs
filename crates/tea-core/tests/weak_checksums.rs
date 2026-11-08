//! `IntegrityOutcome::Weak` and the `ChecksumPolicy` opt-in.

use libtea_core::{
    verify_checksums, verify_checksums_with_policy, Checksum, ChecksumAlgorithm, ChecksumPolicy,
    IntegrityOutcome,
};

// SHA-1("tea")
const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";
// Any 64-hex value that is not SHA-256("tea").
const WRONG_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
// MD5("tea"): 32 hex characters, below the schema's `minLength: 40`.
const MD5_TEA: &str = "7239ea2b5dc943f61f3c0a0276c20974";

fn sha1() -> Checksum {
    Checksum::new(ChecksumAlgorithm::Sha1, SHA1_TEA).unwrap()
}

fn sha256_of(bytes: &[u8]) -> Checksum {
    use sha2::{Digest, Sha256};
    let hex: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Checksum::new(ChecksumAlgorithm::Sha256, &hex).unwrap()
}

#[test]
fn sha1_only_is_weak_by_default() {
    let report = verify_checksums(b"tea", &[sha1()]);
    assert_eq!(report.outcome, IntegrityOutcome::Weak);
    assert!(report.checks[0].weak);
    assert_eq!(report.checks[0].matched, Some(true));
}

#[test]
fn opt_in_turns_a_full_weak_match_into_verified() {
    let policy = ChecksumPolicy::new().with_allow_weak(true);
    let report = verify_checksums_with_policy(b"tea", &[sha1()], policy);
    assert_eq!(report.outcome, IntegrityOutcome::Verified);
    assert!(report.checks[0].weak, "the report still says it was weak");
}

#[test]
fn opt_in_does_not_hide_a_weak_mismatch() {
    let policy = ChecksumPolicy::new().with_allow_weak(true);
    let report = verify_checksums_with_policy(b"other", &[sha1()], policy);
    assert_eq!(report.outcome, IntegrityOutcome::Failed);
}

#[test]
fn weak_plus_strong_match_is_verified_without_opt_in() {
    let report = verify_checksums(b"tea", &[sha1(), sha256_of(b"tea")]);
    assert_eq!(report.outcome, IntegrityOutcome::Verified);
    assert!(report.checks[0].weak);
    assert!(!report.checks[1].weak);
}

#[test]
fn weak_match_does_not_rescue_a_strong_mismatch() {
    let wrong = Checksum::new(ChecksumAlgorithm::Sha256, WRONG_SHA256).unwrap();
    let report = verify_checksums(b"tea", &[sha1(), wrong]);
    assert_eq!(report.outcome, IntegrityOutcome::Failed);
}

#[test]
fn a_real_md5_value_is_rejected_by_the_schema_length() {
    assert!(Checksum::new(ChecksumAlgorithm::from_wire("MD5"), MD5_TEA).is_err());
}

#[test]
fn md5_is_flagged_weak_and_never_computed() {
    // A 40-hex value labelled MD5 passes the schema length but is never computed.
    let md5 = Checksum::new(
        ChecksumAlgorithm::from_wire("md5"),
        "c4ca4238a0b923820dcc509a6f75849bc4ca4238",
    )
    .unwrap();
    assert!(md5.algorithm().is_weak());
    for policy in [
        ChecksumPolicy::new(),
        ChecksumPolicy::new().with_allow_weak(true),
    ] {
        let report = verify_checksums_with_policy(b"tea", std::slice::from_ref(&md5), policy);
        assert_eq!(report.outcome, IntegrityOutcome::Unsupported);
        assert!(report.checks[0].weak);
        assert_eq!(report.checks[0].actual, None);
    }
}

#[test]
fn only_sha1_and_md5_are_weak() {
    assert!(ChecksumAlgorithm::Sha1.is_weak());
    for strong in [
        ChecksumAlgorithm::Sha256,
        ChecksumAlgorithm::Sha384,
        ChecksumAlgorithm::Sha512,
        ChecksumAlgorithm::Sha3_256,
        ChecksumAlgorithm::Blake2b256,
        ChecksumAlgorithm::Blake3,
        ChecksumAlgorithm::from_wire("FUTURE-HASH"),
    ] {
        assert!(!strong.is_weak(), "{strong}");
    }
}
