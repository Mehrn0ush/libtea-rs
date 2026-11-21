//! `KeyVerifier`: raw detached signatures checked against caller-supplied
//! public keys (feature `signature-keys`, using `ring`).

use std::fmt;

use base64::Engine;
use ring::signature::{
    UnparsedPublicKey, ECDSA_P256_SHA256_ASN1, ECDSA_P256_SHA256_FIXED, ED25519,
};
use thiserror::Error;

use super::{
    detect_signature_format, SignatureFormat, SignatureVerification, SignatureVerifier,
    SignatureVerifyOutcome, SignedContent, VerifyRequest,
};

/// DER `SubjectPublicKeyInfo` header of an Ed25519 key (RFC 8410), followed
/// by the 32 key bytes.
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// DER `SubjectPublicKeyInfo` header of a P-256 key (RFC 5480,
/// `id-ecPublicKey` with `prime256v1`), followed by the 65-byte uncompressed
/// point.
const P256_SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

const PEM_BEGIN: &str = "-----BEGIN PUBLIC KEY-----";
const PEM_END: &str = "-----END PUBLIC KEY-----";

/// Key files longer than this are rejected before parsing.
const MAX_KEY_BYTES: usize = 4096;

/// The signature algorithm of a [`PublicKey`].
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyAlgorithm {
    /// Ed25519 (RFC 8032) over the content bytes. The signature is the
    /// 64-byte value, as `openssl pkeyutl -sign -rawin` writes it.
    Ed25519,
    /// ECDSA with P-256 and SHA-256 over the content bytes. The signature is
    /// DER-encoded, as `openssl dgst -sha256 -sign` writes it, or the 64-byte
    /// `r || s` form.
    EcdsaP256Sha256,
}

impl KeyAlgorithm {
    /// A stable lowercase name: `ed25519` or `ecdsa-p256-sha256`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::EcdsaP256Sha256 => "ecdsa-p256-sha256",
        }
    }
}

impl fmt::Display for KeyAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why bytes are not a public key this verifier accepts.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum KeyError {
    /// Longer than 4096 bytes.
    #[error("public key is longer than 4096 bytes")]
    TooLong,
    /// A `PUBLIC KEY` PEM block whose body is not Base64.
    #[error("public key PEM body is not Base64")]
    Pem,
    /// Not one of the accepted encodings of an Ed25519 or P-256 key.
    #[error(
        "not an Ed25519 or P-256 public key (accepted: PEM or DER SubjectPublicKeyInfo, \
         raw 32-byte Ed25519, raw 65-byte uncompressed P-256 point)"
    )]
    Unrecognized,
}

/// A public key the caller trusts.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Accepted encodings, recognised by exact length and fixed header bytes
/// rather than by a general ASN.1 parser:
///
/// - a PEM `PUBLIC KEY` block (as `openssl pkey -pubout` writes it) holding
///   one of the DER forms below;
/// - DER `SubjectPublicKeyInfo` of an Ed25519 key (44 bytes) or a P-256 key
///   with an uncompressed point (91 bytes);
/// - the raw 32-byte Ed25519 key, or the raw 65-byte uncompressed P-256 point.
///
/// [`Self::fingerprint`] is the SHA-256 of the DER `SubjectPublicKeyInfo`,
/// the same value as `openssl pkey -pubin -outform DER | sha256sum`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    algorithm: KeyAlgorithm,
    key: Vec<u8>,
    fingerprint: String,
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublicKey")
            .field("algorithm", &self.algorithm)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl PublicKey {
    /// Parse a public key in one of the accepted encodings.
    ///
    /// ```
    /// use libtea_client::signature::{KeyAlgorithm, PublicKey};
    ///
    /// let key = PublicKey::parse(&[7; 32]).unwrap();
    /// assert_eq!(key.algorithm(), KeyAlgorithm::Ed25519);
    /// assert!(key.fingerprint().starts_with("sha256:"));
    /// assert!(PublicKey::parse(b"not a key").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// [`KeyError`] when the bytes are not an accepted encoding. A P-256 point
    /// that is not on the curve is not detected here; no signature verifies
    /// under it.
    pub fn parse(bytes: &[u8]) -> Result<Self, KeyError> {
        if bytes.len() > MAX_KEY_BYTES {
            return Err(KeyError::TooLong);
        }
        if let Some(der) = pem_body(bytes)? {
            return Self::from_der(&der);
        }
        match bytes.len() {
            32 => Ok(Self::new(KeyAlgorithm::Ed25519, bytes)),
            65 if bytes[0] == 0x04 => Ok(Self::new(KeyAlgorithm::EcdsaP256Sha256, bytes)),
            _ => Self::from_der(bytes),
        }
    }

    fn from_der(der: &[u8]) -> Result<Self, KeyError> {
        if let Some(key) = der.strip_prefix(&ED25519_SPKI_PREFIX) {
            if key.len() == 32 {
                return Ok(Self::new(KeyAlgorithm::Ed25519, key));
            }
        }
        if let Some(point) = der.strip_prefix(&P256_SPKI_PREFIX) {
            if point.len() == 65 && point[0] == 0x04 {
                return Ok(Self::new(KeyAlgorithm::EcdsaP256Sha256, point));
            }
        }
        Err(KeyError::Unrecognized)
    }

    fn new(algorithm: KeyAlgorithm, key: &[u8]) -> Self {
        let prefix: &[u8] = match algorithm {
            KeyAlgorithm::Ed25519 => &ED25519_SPKI_PREFIX,
            KeyAlgorithm::EcdsaP256Sha256 => &P256_SPKI_PREFIX,
        };
        let mut der = prefix.to_vec();
        der.extend_from_slice(key);
        let digest = ring::digest::digest(&ring::digest::SHA256, &der);
        let hex: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
        Self {
            algorithm,
            key: key.to_vec(),
            fingerprint: format!("sha256:{hex}"),
        }
    }

    /// The algorithm.
    #[must_use]
    pub fn algorithm(&self) -> KeyAlgorithm {
        self.algorithm
    }

    /// `sha256:` and the lowercase hex SHA-256 of the DER `SubjectPublicKeyInfo`.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn verifies(&self, message: &[u8], signature: &[u8]) -> bool {
        match self.algorithm {
            KeyAlgorithm::Ed25519 => UnparsedPublicKey::new(&ED25519, &self.key)
                .verify(message, signature)
                .is_ok(),
            KeyAlgorithm::EcdsaP256Sha256 => {
                let fixed = signature.len() == 64
                    && UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, &self.key)
                        .verify(message, signature)
                        .is_ok();
                fixed
                    || UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, &self.key)
                        .verify(message, signature)
                        .is_ok()
            }
        }
    }
}

/// The DER inside a `PUBLIC KEY` PEM block, or `None` when the bytes are not
/// PEM.
fn pem_body(bytes: &[u8]) -> Result<Option<Vec<u8>>, KeyError> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(None);
    };
    let text = text.trim();
    let Some(rest) = text.strip_prefix(PEM_BEGIN) else {
        return Ok(None);
    };
    let body = rest.strip_suffix(PEM_END).ok_or(KeyError::Pem)?;
    let joined: String = body.split_whitespace().collect();
    base64::engine::general_purpose::STANDARD
        .decode(joined)
        .map(Some)
        .map_err(|_| KeyError::Pem)
}

/// A [`super::SignatureVerifier`] for raw detached signatures over the decoded
/// artifact bytes, under public keys the caller supplies.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// It trusts exactly the keys it is given, fetches none, and returns:
///
/// - [`SignatureVerifyOutcome::MissingSignature`] for empty signature bytes;
/// - [`SignatureVerifyOutcome::NoTrustAnchor`] when it has no keys;
/// - [`SignatureVerifyOutcome::ContentUnavailable`] when a
///   [`SignedContent::File`] cannot be read;
/// - [`SignatureVerifyOutcome::Verified`] when the signature verifies under one
///   of the keys, with that key's algorithm and fingerprint in
///   [`SignatureVerification::policy`];
/// - otherwise [`SignatureVerifyOutcome::Unsupported`] when the bytes look like
///   a signature container it does not check (OpenPGP, CMS, a Sigstore
///   bundle, or more than 1 MiB), and [`SignatureVerifyOutcome::Invalid`] for
///   anything else.
///
/// A raw signature is tried under every key before its format is looked at,
/// so bytes that happen to resemble a container still verify.
///
/// ```
/// use libtea_client::signature::{
///     KeyVerifier, PublicKey, SignatureVerifier, SignatureVerifyOutcome, SignedContent,
///     VerifyRequest,
/// };
///
/// let verifier = KeyVerifier::new(vec![PublicKey::parse(&[7; 32]).unwrap()]);
/// let request = VerifyRequest::new(SignedContent::Bytes(b"artifact"), &[0; 64]);
/// assert_eq!(verifier.verify(&request).outcome, SignatureVerifyOutcome::Invalid);
/// assert_eq!(
///     KeyVerifier::new(vec![]).verify(&request).outcome,
///     SignatureVerifyOutcome::NoTrustAnchor
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyVerifier {
    keys: Vec<PublicKey>,
}

impl KeyVerifier {
    /// A verifier that trusts exactly `keys`.
    #[must_use]
    pub fn new(keys: Vec<PublicKey>) -> Self {
        Self { keys }
    }

    /// The trusted keys.
    #[must_use]
    pub fn keys(&self) -> &[PublicKey] {
        &self.keys
    }

    fn policy(&self) -> String {
        let fingerprints: Vec<_> = self.keys.iter().map(PublicKey::fingerprint).collect();
        format!(
            "caller keys ({}): {}",
            self.keys.len(),
            fingerprints.join(", ")
        )
    }
}

impl SignatureVerifier for KeyVerifier {
    fn verify(&self, request: &VerifyRequest<'_>) -> SignatureVerification {
        if request.signature.is_empty() {
            return SignatureVerification::new(
                SignatureVerifyOutcome::MissingSignature,
                self.policy(),
            );
        }
        if self.keys.is_empty() {
            return SignatureVerification::new(
                SignatureVerifyOutcome::NoTrustAnchor,
                "caller keys (0)",
            );
        }
        let read;
        let message: &[u8] = match request.content {
            SignedContent::Bytes(bytes) => bytes,
            SignedContent::File(path) => match std::fs::read(path) {
                Ok(bytes) => {
                    read = bytes;
                    &read
                }
                Err(error) => {
                    return SignatureVerification::new(
                        SignatureVerifyOutcome::ContentUnavailable,
                        self.policy(),
                    )
                    .with_detail(format!("cannot read {}: {error}", path.display()));
                }
            },
        };
        if let Some(key) = self
            .keys
            .iter()
            .find(|key| key.verifies(message, request.signature))
        {
            return SignatureVerification::new(
                SignatureVerifyOutcome::Verified,
                format!("caller key {} {}", key.algorithm(), key.fingerprint()),
            );
        }
        let format = detect_signature_format(request.signature);
        match format {
            SignatureFormat::OpenPgpArmored
            | SignatureFormat::OpenPgpBinary
            | SignatureFormat::Cms
            | SignatureFormat::SigstoreBundle
            | SignatureFormat::TooLarge => {
                SignatureVerification::new(SignatureVerifyOutcome::Unsupported, self.policy())
                    .with_detail(format!(
                    "signature looks like {}, which this verifier does not check; it checks raw \
                         Ed25519 and ECDSA P-256 signatures",
                    format.as_str()
                ))
            }
            _ => SignatureVerification::new(SignatureVerifyOutcome::Invalid, self.policy())
                .with_detail("the signature does not verify under any of the keys"),
        }
    }
}

#[cfg(test)]
mod tests {
    use ring::rand::SystemRandom;
    use ring::signature::{EcdsaKeyPair, Ed25519KeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

    use super::*;

    fn ed25519() -> Ed25519KeyPair {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
    }

    fn p256() -> EcdsaKeyPair {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap()
    }

    fn check(verifier: &KeyVerifier, message: &[u8], signature: &[u8]) -> SignatureVerification {
        verifier.verify(&VerifyRequest::new(
            SignedContent::Bytes(message),
            signature,
        ))
    }

    #[test]
    fn ed25519_signatures_verify_only_for_their_content_and_key() {
        let pair = ed25519();
        let key = PublicKey::parse(pair.public_key().as_ref()).unwrap();
        assert_eq!(key.algorithm(), KeyAlgorithm::Ed25519);
        let signature = pair.sign(b"artifact");
        let verifier = KeyVerifier::new(vec![key.clone()]);
        let result = check(&verifier, b"artifact", signature.as_ref());
        assert_eq!(result.outcome, SignatureVerifyOutcome::Verified);
        assert_eq!(
            result.policy,
            format!("caller key ed25519 {}", key.fingerprint())
        );
        assert_eq!(
            check(&verifier, b"artifacT", signature.as_ref()).outcome,
            SignatureVerifyOutcome::Invalid
        );
        let other = KeyVerifier::new(vec![
            PublicKey::parse(ed25519().public_key().as_ref()).unwrap()
        ]);
        assert_eq!(
            check(&other, b"artifact", signature.as_ref()).outcome,
            SignatureVerifyOutcome::Invalid
        );
    }

    #[test]
    fn p256_signatures_verify_in_der_and_fixed_form() {
        let rng = SystemRandom::new();
        let pair = p256();
        let key = PublicKey::parse(pair.public_key().as_ref()).unwrap();
        assert_eq!(key.algorithm(), KeyAlgorithm::EcdsaP256Sha256);
        let der = pair.sign(&rng, b"artifact").unwrap();
        let verifier = KeyVerifier::new(vec![key]);
        assert_eq!(
            check(&verifier, b"artifact", der.as_ref()).outcome,
            SignatureVerifyOutcome::Verified
        );
        let fixed_pair = {
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(
                &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                &rng,
            )
            .unwrap();
            EcdsaKeyPair::from_pkcs8(
                &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                pkcs8.as_ref(),
                &rng,
            )
            .unwrap()
        };
        let fixed = fixed_pair.sign(&rng, b"artifact").unwrap();
        assert_eq!(fixed.as_ref().len(), 64);
        let verifier = KeyVerifier::new(vec![
            PublicKey::parse(fixed_pair.public_key().as_ref()).unwrap()
        ]);
        assert_eq!(
            check(&verifier, b"artifact", fixed.as_ref()).outcome,
            SignatureVerifyOutcome::Verified
        );
        assert_eq!(
            check(&verifier, b"other", fixed.as_ref()).outcome,
            SignatureVerifyOutcome::Invalid
        );
    }

    #[test]
    fn any_of_several_keys_may_verify() {
        let pair = ed25519();
        let keys = vec![
            PublicKey::parse(p256().public_key().as_ref()).unwrap(),
            PublicKey::parse(pair.public_key().as_ref()).unwrap(),
        ];
        let verifier = KeyVerifier::new(keys);
        let result = check(&verifier, b"artifact", pair.sign(b"artifact").as_ref());
        assert_eq!(result.outcome, SignatureVerifyOutcome::Verified);
        assert!(result.policy.starts_with("caller key ed25519 sha256:"));
    }

    #[test]
    fn outcomes_other_than_verified() {
        let verifier = KeyVerifier::new(vec![
            PublicKey::parse(ed25519().public_key().as_ref()).unwrap()
        ]);
        assert_eq!(
            check(&verifier, b"artifact", b"").outcome,
            SignatureVerifyOutcome::MissingSignature
        );
        assert_eq!(
            check(&KeyVerifier::new(vec![]), b"artifact", b"x").outcome,
            SignatureVerifyOutcome::NoTrustAnchor
        );
        let armored = b"-----BEGIN PGP SIGNATURE-----\n\nabc\n-----END PGP SIGNATURE-----\n";
        let result = check(&verifier, b"artifact", armored);
        assert_eq!(result.outcome, SignatureVerifyOutcome::Unsupported);
        assert!(result.detail.unwrap().contains("openpgp"));
        assert_eq!(
            check(&verifier, b"artifact", &vec![0; 2 << 20]).outcome,
            SignatureVerifyOutcome::Unsupported
        );
        assert_eq!(
            check(&verifier, b"artifact", &[1; 64]).outcome,
            SignatureVerifyOutcome::Invalid
        );
        let missing = std::path::Path::new("/nonexistent/libtea/artifact");
        let result = verifier.verify(&VerifyRequest::new(SignedContent::File(missing), &[1; 64]));
        assert_eq!(result.outcome, SignatureVerifyOutcome::ContentUnavailable);
    }

    #[test]
    fn key_encodings() {
        let pair = ed25519();
        let raw = pair.public_key().as_ref();
        let mut der = ED25519_SPKI_PREFIX.to_vec();
        der.extend_from_slice(raw);
        let pem = format!(
            "{PEM_BEGIN}\n{}\n{PEM_END}\n",
            base64::engine::general_purpose::STANDARD.encode(&der)
        );
        let from_raw = PublicKey::parse(raw).unwrap();
        assert_eq!(PublicKey::parse(&der).unwrap(), from_raw);
        assert_eq!(PublicKey::parse(pem.as_bytes()).unwrap(), from_raw);
        for bad in [
            &b""[..],
            &[0u8; 31],
            &[0u8; 33],
            &[0x05; 65],
            &der[..43],
            &[der.as_slice(), &[0]].concat(),
        ] {
            assert_eq!(
                PublicKey::parse(bad),
                Err(KeyError::Unrecognized),
                "{bad:?}"
            );
        }
        assert_eq!(
            PublicKey::parse(format!("{PEM_BEGIN}\n!!\n{PEM_END}").as_bytes()),
            Err(KeyError::Pem)
        );
        assert_eq!(
            PublicKey::parse(format!("{PEM_BEGIN}\nAAAA\n").as_bytes()),
            Err(KeyError::Pem)
        );
        assert_eq!(PublicKey::parse(&[0; 4097]), Err(KeyError::TooLong));
        // The Debug output names the key, not its bytes.
        assert!(!format!("{from_raw:?}").contains("key:"));
    }
}
