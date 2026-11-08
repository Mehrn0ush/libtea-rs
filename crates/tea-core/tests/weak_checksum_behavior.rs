//! A match on weak algorithms only must not be reported as `Verified`.
//!
//! Uses only API that existed before `IntegrityOutcome::Weak`, so it also runs
//! against older trees.

use libtea_core::{verify_checksums, Checksum, ChecksumAlgorithm, IntegrityOutcome};

// SHA-1("tea")
const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";

#[test]
fn sha1_only_match_is_not_verified_by_default() {
    let checksum = Checksum::new(ChecksumAlgorithm::Sha1, SHA1_TEA).unwrap();
    let report = verify_checksums(b"tea", &[checksum]);
    assert_ne!(report.outcome, IntegrityOutcome::Verified);
}

#[test]
fn md5_labelled_only_is_not_verified() {
    let md5 = Checksum::new(
        ChecksumAlgorithm::from_wire("MD5"),
        "c4ca4238a0b923820dcc509a6f75849bc4ca4238",
    )
    .unwrap();
    assert_ne!(
        verify_checksums(b"tea", &[md5]).outcome,
        IntegrityOutcome::Verified
    );
}
