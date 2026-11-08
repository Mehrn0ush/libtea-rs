//! Extension point for cryptographic signature verification.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! **0.1.0 ships no verifier that returns [`SignatureVerifyOutcome::Verified`].**
//! Checksum [`libtea_core::IntegrityOutcome::Verified`] means declared digests
//! matched decoded artifact bytes. It does **not** mean a publisher signed the
//! bytes or that a trust policy was evaluated.
//!
//! See `docs/design/signature-verification.md`.

/// Outcome of an optional cryptographic signature check.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Callers must not treat anything other than [`Self::Verified`] as authenticity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureVerifyOutcome {
    /// Signature validated under a caller-supplied trust policy.
    ///
    /// No built-in verifier in this crate returns this variant.
    Verified,
    /// Bytes were present and parseable but failed cryptographic checks.
    Invalid,
    /// Format or algorithm is not supported by this verifier.
    Unsupported,
    /// Verifier has no trust anchors / policy configured.
    NoTrustAnchor,
    /// No signature bytes were supplied.
    MissingSignature,
}

/// Caller-supplied cryptographic verifier.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Implementations live outside this crate (or in tests). The HTTP client does
/// not invoke this trait automatically in 0.1.0.
pub trait SignatureVerifier: Send + Sync {
    /// Inspect artifact bytes and detached signature bytes.
    fn verify(
        &self,
        artifact: &[u8],
        signature: &[u8],
        media_hint: Option<&str>,
    ) -> SignatureVerifyOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysUnsupported;

    impl SignatureVerifier for AlwaysUnsupported {
        fn verify(
            &self,
            _artifact: &[u8],
            _signature: &[u8],
            _media_hint: Option<&str>,
        ) -> SignatureVerifyOutcome {
            SignatureVerifyOutcome::Unsupported
        }
    }

    #[test]
    fn test_double_never_claims_verified() {
        let outcome = AlwaysUnsupported.verify(b"artifact", b"sig", Some("application/json"));
        assert_ne!(outcome, SignatureVerifyOutcome::Verified);
        assert_eq!(outcome, SignatureVerifyOutcome::Unsupported);
    }
}
