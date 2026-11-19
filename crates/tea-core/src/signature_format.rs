//! Recognize the format of detached signature bytes, without verifying them.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! TEA "makes no assumption about the signature technology in use"
//! (`openapi.yaml`, `/artifact/{uuid}/{artifactVersion}/signature/download`).
//! This module looks at the bytes a server returned and says which known
//! container they look like. That is all: a recognized format is **not** a
//! valid signature, and nothing here checks a signature, a key or a signer.
//! The server's `Content-Type` is not consulted, so a mislabelled response
//! cannot change the answer.
//!
//! See `docs/design/signature-verification.md`, "Stage 1".

/// Inputs longer than this are not inspected ([`SignatureFormat::TooLarge`]).
pub const MAX_SIGNATURE_INSPECT_BYTES: usize = 1024 * 1024;

/// The container a detached signature appears to use.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SignatureFormat {
    /// OpenPGP ASCII armor (`-----BEGIN PGP SIGNATURE-----`).
    OpenPgpArmored,
    /// A binary OpenPGP signature packet (tag 2, version 3 to 6).
    OpenPgpBinary,
    /// DER CMS `ContentInfo` whose content type is `SignedData`
    /// (OID 1.2.840.113549.1.7.2).
    Cms,
    /// A JSON object whose `mediaType` starts with
    /// `application/vnd.dev.sigstore.bundle`.
    SigstoreBundle,
    /// No bytes.
    Empty,
    /// More than [`MAX_SIGNATURE_INSPECT_BYTES`]; not inspected.
    TooLarge,
    /// None of the above.
    Unknown,
}

impl SignatureFormat {
    /// Stable lowercase name, used in CLI JSON output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenPgpArmored => "openpgp_armored",
            Self::OpenPgpBinary => "openpgp_binary",
            Self::Cms => "cms",
            Self::SigstoreBundle => "sigstore_bundle",
            Self::Empty => "empty",
            Self::TooLarge => "too_large",
            Self::Unknown => "unknown",
        }
    }
}

/// Say which signature container `bytes` look like.
///
/// Never panics. Only the first bytes are read for the binary formats; the
/// Sigstore check parses the whole input as JSON, which is why the input is
/// capped at [`MAX_SIGNATURE_INSPECT_BYTES`] first.
///
/// ```
/// use libtea_core::signature_format::{detect_signature_format, SignatureFormat};
///
/// let armored = b"-----BEGIN PGP SIGNATURE-----\n\niQEz...\n-----END PGP SIGNATURE-----\n";
/// assert_eq!(detect_signature_format(armored), SignatureFormat::OpenPgpArmored);
/// assert_eq!(detect_signature_format(b""), SignatureFormat::Empty);
/// assert_eq!(detect_signature_format(b"not a signature"), SignatureFormat::Unknown);
/// ```
#[must_use]
pub fn detect_signature_format(bytes: &[u8]) -> SignatureFormat {
    if bytes.is_empty() {
        return SignatureFormat::Empty;
    }
    if bytes.len() > MAX_SIGNATURE_INSPECT_BYTES {
        return SignatureFormat::TooLarge;
    }
    if is_openpgp_armored(bytes) {
        SignatureFormat::OpenPgpArmored
    } else if is_openpgp_binary(bytes) {
        SignatureFormat::OpenPgpBinary
    } else if is_cms_signed_data(bytes) {
        SignatureFormat::Cms
    } else if is_sigstore_bundle(bytes) {
        SignatureFormat::SigstoreBundle
    } else {
        SignatureFormat::Unknown
    }
}

const PGP_ARMOR: &[u8] = b"-----BEGIN PGP SIGNATURE-----";

fn is_openpgp_armored(bytes: &[u8]) -> bool {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let rest = &bytes[start..];
    rest.starts_with(PGP_ARMOR) && matches!(rest.get(PGP_ARMOR.len()), Some(b'\r' | b'\n'))
}

/// RFC 9580 section 4.2: the packet header, then a signature packet body that
/// starts with its version octet. The declared length must fit the input.
fn is_openpgp_binary(bytes: &[u8]) -> bool {
    let Some(&header) = bytes.first() else {
        return false;
    };
    if header & 0x80 == 0 {
        return false;
    }
    let (tag, body_start, body_len) = if header & 0x40 != 0 {
        // New format: the tag is the low six bits.
        let Some(&first) = bytes.get(1) else {
            return false;
        };
        let (start, len) = match first {
            0..=191 => (2, usize::from(first)),
            192..=223 => match bytes.get(2) {
                Some(&second) => (
                    3,
                    ((usize::from(first) - 192) << 8) + usize::from(second) + 192,
                ),
                None => return false,
            },
            255 => match bytes.get(2..6) {
                Some(four) => (6, be_len(four)),
                None => return false,
            },
            // Partial body lengths are not allowed for signature packets.
            _ => return false,
        };
        (header & 0x3f, start, len)
    } else {
        // Legacy format: tag in bits 5..2, length type in bits 1..0.
        let length_octets = match header & 0x03 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => return false,
        };
        match bytes.get(1..1 + length_octets) {
            Some(octets) => ((header >> 2) & 0x0f, 1 + length_octets, be_len(octets)),
            None => return false,
        }
    };
    tag == 2
        && body_len >= 1
        && bytes.len() - body_start >= body_len
        && matches!(bytes.get(body_start), Some(3..=6))
}

fn be_len(octets: &[u8]) -> usize {
    octets.iter().fold(0usize, |acc, &b| {
        acc.saturating_mul(256).saturating_add(usize::from(b))
    })
}

/// DER encoding of OID 1.2.840.113549.1.7.2 (`id-signedData`), tag and length included.
const SIGNED_DATA_OID: &[u8] = &[
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02,
];

/// `ContentInfo ::= SEQUENCE { contentType OBJECT IDENTIFIER, ... }`: a SEQUENCE
/// whose definite length fits the input, starting with the `SignedData` OID.
fn is_cms_signed_data(bytes: &[u8]) -> bool {
    if bytes.first() != Some(&0x30) {
        return false;
    }
    let Some(&first) = bytes.get(1) else {
        return false;
    };
    let (content_start, len) = if first < 0x80 {
        (2, usize::from(first))
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 {
            return false;
        }
        match bytes.get(2..2 + count) {
            Some(octets) => (2 + count, be_len(octets)),
            None => return false,
        }
    };
    len >= SIGNED_DATA_OID.len()
        && bytes.len() - content_start >= len
        && bytes[content_start..].starts_with(SIGNED_DATA_OID)
}

const SIGSTORE_BUNDLE_PREFIX: &str = "application/vnd.dev.sigstore.bundle";

fn is_sigstore_bundle(bytes: &[u8]) -> bool {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    if bytes.get(start) != Some(&b'{') {
        return false;
    }
    #[derive(serde::Deserialize)]
    struct Head {
        #[serde(rename = "mediaType")]
        media_type: Option<String>,
    }
    serde_json::from_slice::<Head>(bytes)
        .ok()
        .and_then(|head| head.media_type)
        .is_some_and(|media_type| media_type.starts_with(SIGSTORE_BUNDLE_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A v4 signature packet: legacy header (tag 2, one length octet), body
    /// starting with the version octet. Not a valid signature.
    fn pgp_legacy(version: u8) -> Vec<u8> {
        let body = [version, 0x00, 0x01, 0x08, 0x00, 0x00];
        let mut packet = vec![0x88, body.len() as u8];
        packet.extend_from_slice(&body);
        packet
    }

    fn cms(extra: &[u8]) -> Vec<u8> {
        let mut content = SIGNED_DATA_OID.to_vec();
        content.extend_from_slice(extra);
        let mut der = vec![0x30, content.len() as u8];
        der.extend_from_slice(&content);
        der
    }

    #[test]
    fn armored_openpgp_with_leading_whitespace_and_crlf() {
        assert_eq!(
            detect_signature_format(b"\r\n  -----BEGIN PGP SIGNATURE-----\r\n\r\nabc"),
            SignatureFormat::OpenPgpArmored
        );
        // A signed message is not a detached signature.
        assert_eq!(
            detect_signature_format(b"-----BEGIN PGP MESSAGE-----\n"),
            SignatureFormat::Unknown
        );
        // The armor line must end there.
        assert_eq!(
            detect_signature_format(b"-----BEGIN PGP SIGNATURE-----X\n"),
            SignatureFormat::Unknown
        );
    }

    #[test]
    fn binary_openpgp_legacy_and_new_headers() {
        assert_eq!(
            detect_signature_format(&pgp_legacy(4)),
            SignatureFormat::OpenPgpBinary
        );
        assert_eq!(
            detect_signature_format(&pgp_legacy(6)),
            SignatureFormat::OpenPgpBinary
        );
        // New format, tag 2, one-octet length.
        assert_eq!(
            detect_signature_format(&[0xc2, 0x03, 0x04, 0x00, 0x01]),
            SignatureFormat::OpenPgpBinary
        );
        // New format, two-octet length 192 + 1 = 193.
        let mut long = vec![0xc2, 0xc0, 0x01, 0x04];
        long.resize(3 + 193, 0);
        assert_eq!(
            detect_signature_format(&long),
            SignatureFormat::OpenPgpBinary
        );
        long.truncate(long.len() - 1);
        assert_eq!(detect_signature_format(&long), SignatureFormat::Unknown);
    }

    #[test]
    fn binary_openpgp_rejects_other_packets_and_bad_lengths() {
        // Unknown signature version.
        assert_eq!(
            detect_signature_format(&pgp_legacy(7)),
            SignatureFormat::Unknown
        );
        // Tag 6 (public key), legacy header 0x98.
        assert_eq!(
            detect_signature_format(&[0x98, 0x02, 0x04, 0x00]),
            SignatureFormat::Unknown
        );
        // Declared length longer than the input.
        assert_eq!(
            detect_signature_format(&[0x88, 0x40, 0x04, 0x00]),
            SignatureFormat::Unknown
        );
        // Indeterminate legacy length and partial new-format length.
        assert_eq!(
            detect_signature_format(&[0x8b, 0x04, 0x00]),
            SignatureFormat::Unknown
        );
        assert_eq!(
            detect_signature_format(&[0xc2, 0xe0, 0x04]),
            SignatureFormat::Unknown
        );
        assert_eq!(detect_signature_format(&[0x88]), SignatureFormat::Unknown);
        assert_eq!(
            detect_signature_format(&[0xc2, 0xff, 0x00]),
            SignatureFormat::Unknown
        );
    }

    #[test]
    fn cms_signed_data_short_and_long_lengths() {
        assert_eq!(
            detect_signature_format(&cms(&[0xa0, 0x00])),
            SignatureFormat::Cms
        );
        let mut long = vec![0x30, 0x81, 0x80];
        long.extend_from_slice(SIGNED_DATA_OID);
        long.resize(3 + 0x80, 0);
        assert_eq!(detect_signature_format(&long), SignatureFormat::Cms);
        long.pop();
        assert_eq!(detect_signature_format(&long), SignatureFormat::Unknown);
    }

    #[test]
    fn cms_with_another_content_type_is_unknown() {
        // id-data (1.2.840.113549.1.7.1).
        let mut data = cms(&[]);
        let last = data.len() - 1;
        data[last] = 0x01;
        assert_eq!(detect_signature_format(&data), SignatureFormat::Unknown);
        assert_eq!(
            detect_signature_format(&[0x30, 0x80]),
            SignatureFormat::Unknown
        );
        assert_eq!(
            detect_signature_format(&[0x30, 0x85, 1, 1, 1, 1, 1]),
            SignatureFormat::Unknown
        );
    }

    #[test]
    fn sigstore_bundle_by_media_type_only() {
        let v03 = br#"{"mediaType":"application/vnd.dev.sigstore.bundle.v0.3+json","verificationMaterial":{}}"#;
        let v01 = br#" {"mediaType":"application/vnd.dev.sigstore.bundle+json;version=0.1"}"#;
        assert_eq!(
            detect_signature_format(v03),
            SignatureFormat::SigstoreBundle
        );
        assert_eq!(
            detect_signature_format(v01),
            SignatureFormat::SigstoreBundle
        );
        for other in [
            br#"{"mediaType":"application/json"}"#.as_slice(),
            br#"{"verificationMaterial":{}}"#,
            br#"{"mediaType":7}"#,
            br#"["application/vnd.dev.sigstore.bundle"]"#,
            br#"{"mediaType":"application/vnd.dev.sigstore.bundle""#,
        ] {
            assert_eq!(detect_signature_format(other), SignatureFormat::Unknown);
        }
    }

    #[test]
    fn empty_and_oversized_are_not_inspected() {
        assert_eq!(detect_signature_format(b""), SignatureFormat::Empty);
        let mut big = PGP_ARMOR.to_vec();
        big.push(b'\n');
        big.resize(MAX_SIGNATURE_INSPECT_BYTES + 1, b'A');
        assert_eq!(detect_signature_format(&big), SignatureFormat::TooLarge);
        big.truncate(MAX_SIGNATURE_INSPECT_BYTES);
        assert_eq!(
            detect_signature_format(&big),
            SignatureFormat::OpenPgpArmored
        );
    }

    #[test]
    fn names_are_distinct() {
        let all = [
            SignatureFormat::OpenPgpArmored,
            SignatureFormat::OpenPgpBinary,
            SignatureFormat::Cms,
            SignatureFormat::SigstoreBundle,
            SignatureFormat::Empty,
            SignatureFormat::TooLarge,
            SignatureFormat::Unknown,
        ];
        let names: std::collections::BTreeSet<_> = all.iter().map(|f| f.as_str()).collect();
        assert_eq!(names.len(), all.len());
    }
}
