//! Cryptographic signature verification: the verifier trait, a verifier that
//! checks nothing, and (feature `signature-keys`) one that checks raw detached
//! signatures against public keys the caller supplies.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! Checksum [`libtea_core::IntegrityOutcome::Verified`] means declared digests
//! matched decoded artifact bytes. It does **not** mean a publisher signed the
//! bytes. Only [`SignatureVerifyOutcome::Verified`] says that, and only a
//! verifier configured by the caller with keys it trusts returns it: nothing
//! in this crate fetches keys or trusts a key by default.
//!
//! See `docs/design/signature-verification.md`.
//!
//! [`detect_signature_format`] (re-exported from `libtea-core`) names the
//! container a downloaded signature appears to use. [`FormatOnlyVerifier`]
//! never verifies. `KeyVerifier` (feature `signature-keys`) verifies.

use std::path::Path;

pub use libtea_core::signature_format::{
    detect_signature_format, SignatureFormat, MAX_SIGNATURE_INSPECT_BYTES,
};

#[cfg(feature = "signature-keys")]
mod keys;
#[cfg(feature = "signature-keys")]
pub use keys::{KeyAlgorithm, KeyError, KeyVerifier, PublicKey};

/// Outcome of a signature check.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Callers must not treat anything other than [`Self::Verified`] as authenticity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureVerifyOutcome {
    /// The signature is valid for the content under a key the caller supplied.
    /// [`SignatureVerification::policy`] names the key.
    Verified,
    /// The signature was checked and does not match the content under any key.
    Invalid,
    /// The signature's format or algorithm is not one this verifier checks.
    /// The specification asks for such a signature to be treated "as unusable
    /// rather than as invalid".
    Unsupported,
    /// The verifier has no keys or policy configured.
    NoTrustAnchor,
    /// No signature bytes were supplied.
    MissingSignature,
    /// The signed content could not be read (for example a file that is gone).
    ContentUnavailable,
}

impl SignatureVerifyOutcome {
    /// A stable snake_case name, as in `libtea download --json`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Invalid => "invalid",
            Self::Unsupported => "unsupported",
            Self::NoTrustAnchor => "no_trust_anchor",
            Self::MissingSignature => "missing_signature",
            Self::ContentUnavailable => "content_unavailable",
        }
    }
}

/// What a detached signature is checked against: the decoded artifact bytes,
/// the same bytes the checksums cover.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum SignedContent<'a> {
    /// The bytes, in memory.
    Bytes(&'a [u8]),
    /// The bytes, in a file, for example the one
    /// [`crate::Service::download_format_to_file`] wrote. A verifier reads it
    /// itself; `KeyVerifier` reads it whole, because the signature algorithms
    /// it supports take the whole message.
    File(&'a Path),
}

/// One verification request.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VerifyRequest<'a> {
    /// What was signed.
    pub content: SignedContent<'a>,
    /// The detached signature bytes, as downloaded.
    pub signature: &'a [u8],
    /// The `mediaType` of the artifact format the signature belongs to (it
    /// selects the artifact format, not the signature's own type).
    pub media_type: Option<&'a str>,
}

impl<'a> VerifyRequest<'a> {
    /// A request for `signature` over `content`, without a media type.
    #[must_use]
    pub fn new(content: SignedContent<'a>, signature: &'a [u8]) -> Self {
        Self {
            content,
            signature,
            media_type: None,
        }
    }

    /// Set the artifact format's media type.
    #[must_use]
    pub fn with_media_type(mut self, media_type: &'a str) -> Self {
        self.media_type = Some(media_type);
        self
    }
}

/// The result of [`SignatureVerifier::verify`]: the outcome, and which
/// verifier and policy produced it, so a result can be audited later.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SignatureVerification {
    /// The outcome.
    pub outcome: SignatureVerifyOutcome,
    /// The verifier and the policy it applied, for example
    /// `caller key ed25519 sha256:3b6a…` for a [`SignatureVerifyOutcome::Verified`]
    /// result: the key that verified, by the SHA-256 of its DER
    /// `SubjectPublicKeyInfo`.
    pub policy: String,
    /// Why the outcome is not `Verified`, when there is more to say.
    pub detail: Option<String>,
}

impl SignatureVerification {
    /// A result with no detail.
    #[must_use]
    pub fn new(outcome: SignatureVerifyOutcome, policy: impl Into<String>) -> Self {
        Self {
            outcome,
            policy: policy.into(),
            detail: None,
        }
    }

    /// Add a detail.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// A signature verifier configured by the caller.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// The HTTP client never calls it by itself. An implementation must return
/// [`SignatureVerifyOutcome::Verified`] only under a policy the caller
/// configured, and must never derive trust from the server that served the
/// artifact (for example from a key it also served).
pub trait SignatureVerifier: Send + Sync {
    /// Check `request.signature` over `request.content`.
    fn verify(&self, request: &VerifyRequest<'_>) -> SignatureVerification;
}

/// A [`SignatureVerifier`] that checks nothing and says so.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Returns [`SignatureVerifyOutcome::MissingSignature`] for empty signature
/// bytes and [`SignatureVerifyOutcome::Unsupported`] for anything else,
/// whatever format [`detect_signature_format`] finds. It never returns
/// [`SignatureVerifyOutcome::Verified`].
///
/// ```
/// use libtea_client::signature::{
///     FormatOnlyVerifier, SignatureVerifier, SignatureVerifyOutcome, SignedContent, VerifyRequest,
/// };
///
/// let verifier = FormatOnlyVerifier::new();
/// let content = SignedContent::Bytes(b"artifact");
/// let result = verifier.verify(&VerifyRequest::new(content, b""));
/// assert_eq!(result.outcome, SignatureVerifyOutcome::MissingSignature);
/// let result = verifier.verify(&VerifyRequest::new(content, b"sig"));
/// assert_eq!(result.outcome, SignatureVerifyOutcome::Unsupported);
/// assert_eq!(result.policy, "format only: no signature is checked");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormatOnlyVerifier {}

impl FormatOnlyVerifier {
    /// The verifier.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl SignatureVerifier for FormatOnlyVerifier {
    fn verify(&self, request: &VerifyRequest<'_>) -> SignatureVerification {
        let outcome = if request.signature.is_empty() {
            SignatureVerifyOutcome::MissingSignature
        } else {
            SignatureVerifyOutcome::Unsupported
        };
        SignatureVerification::new(outcome, "format only: no signature is checked")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysUnsupported;

    impl SignatureVerifier for AlwaysUnsupported {
        fn verify(&self, _request: &VerifyRequest<'_>) -> SignatureVerification {
            SignatureVerification::new(SignatureVerifyOutcome::Unsupported, "test double")
        }
    }

    #[test]
    fn format_only_verifier_never_claims_verified() {
        let verifier = FormatOnlyVerifier::new();
        let armored = b"-----BEGIN PGP SIGNATURE-----\n\nabc\n-----END PGP SIGNATURE-----\n";
        assert_eq!(
            detect_signature_format(armored),
            SignatureFormat::OpenPgpArmored
        );
        let content = SignedContent::Bytes(b"artifact");
        for signature in [armored.as_slice(), b"x", b"{}"] {
            let request =
                VerifyRequest::new(content, signature).with_media_type("application/pgp-signature");
            assert_eq!(
                verifier.verify(&request).outcome,
                SignatureVerifyOutcome::Unsupported
            );
        }
        assert_eq!(
            verifier.verify(&VerifyRequest::new(content, b"")).outcome,
            SignatureVerifyOutcome::MissingSignature
        );
    }

    #[test]
    fn test_double_never_claims_verified() {
        let request = VerifyRequest::new(SignedContent::Bytes(b"artifact"), b"sig");
        let result = AlwaysUnsupported.verify(&request);
        assert_ne!(result.outcome, SignatureVerifyOutcome::Verified);
        assert_eq!(result.outcome, SignatureVerifyOutcome::Unsupported);
    }

    #[test]
    fn outcome_names_are_stable() {
        let all = [
            (SignatureVerifyOutcome::Verified, "verified"),
            (SignatureVerifyOutcome::Invalid, "invalid"),
            (SignatureVerifyOutcome::Unsupported, "unsupported"),
            (SignatureVerifyOutcome::NoTrustAnchor, "no_trust_anchor"),
            (
                SignatureVerifyOutcome::MissingSignature,
                "missing_signature",
            ),
            (
                SignatureVerifyOutcome::ContentUnavailable,
                "content_unavailable",
            ),
        ];
        for (outcome, name) in all {
            assert_eq!(outcome.as_str(), name);
        }
    }
}
