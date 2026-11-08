//! Transparency Exchange Identifier parsing and type-specific validation.
//!
//! Parsing accepts a structurally valid TEI and retains the original spelling.
//! Registry checks are separate and do not rewrite the identifier. Equality is
//! exact string equality, as required by the discovery document.

use std::fmt;

use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::enums::ChecksumAlgorithm;
use crate::error::TeiError;
use crate::ids::TeaUuid;

const SCHEME: &str = "tei://";

/// A parsed TEI that still carries its original character sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tei {
    original: String,
    domain: String,
    tei_type: String,
    identifier: String,
}

impl Tei {
    /// Parse `tei://<domain-name>/<type>/<unique-identifier>`.
    ///
    /// The returned value displays as the input. This function does not
    /// lowercase the domain, decode Base64URL, or rewrite percent-escapes.
    ///
    /// # Errors
    ///
    /// Returns an error when the text is not a structurally valid TEI.
    pub fn parse(input: &str) -> Result<Self, TeiError> {
        if input.is_empty() {
            return Err(TeiError::Empty);
        }
        if input
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("urn:tei:"))
        {
            return Err(TeiError::LegacyUrn);
        }
        let rest = input.strip_prefix(SCHEME).ok_or(TeiError::Scheme)?;
        if rest.contains(['?', '#', ' ']) {
            return Err(TeiError::Structure);
        }
        let mut parts = rest.split('/');
        let domain = parts.next().ok_or(TeiError::Structure)?;
        let tei_type = parts.next().ok_or(TeiError::Structure)?;
        let identifier = parts.next().ok_or(TeiError::Structure)?;
        if parts.next().is_some()
            || domain.is_empty()
            || tei_type.is_empty()
            || identifier.is_empty()
        {
            return Err(TeiError::Structure);
        }
        validate_domain(domain)?;
        validate_type(tei_type)?;
        validate_identifier_segment(identifier)?;
        Ok(Self {
            original: input.to_owned(),
            domain: domain.to_owned(),
            tei_type: tei_type.to_owned(),
            identifier: identifier.to_owned(),
        })
    }

    /// Original spelling, used for equality and for the `tei` query parameter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// DNS name carried by the TEI. This is not normalized.
    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Registered type segment.
    #[must_use]
    pub fn tei_type(&self) -> &str {
        &self.tei_type
    }

    /// Unique-identifier segment, still in its TEI spelling.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// `https://<domain>/.well-known/tea`, using the domain spelling as written.
    #[must_use]
    pub fn well_known_url(&self) -> String {
        format!("https://{}/.well-known/tea", self.domain)
    }

    /// Check the unique identifier against the TEI type registry at the pinned commit.
    ///
    /// Unknown types are structurally valid and return `Ok(())`. Their
    /// identifiers are not given extra syntax beyond the single path segment.
    ///
    /// # Errors
    ///
    /// Returns an error when a registered type's identifier syntax is not met.
    pub fn validate_registry(&self) -> Result<(), TeiError> {
        match self.tei_type.as_str() {
            "purl" => validate_purl(&self.identifier),
            "hash" => validate_hash(&self.identifier),
            "uuid" => validate_uuid_id(&self.identifier),
            "eanupc" => validate_eanupc(&self.identifier),
            "gtin" => validate_gtin(&self.identifier),
            "asin" => validate_asin(&self.identifier),
            "udi" => validate_udi(&self.identifier),
            _ => Ok(()),
        }
    }

    /// Publication-form issues that do not make the TEI unparsable.
    ///
    /// The discovery document requires vendors to publish a canonical spelling
    /// and requires clients not to rewrite a TEI they received. These issues
    /// are reported separately from [`Self::parse`].
    #[must_use]
    pub fn publication_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.domain.chars().any(|c| c.is_ascii_uppercase()) {
            issues.push("domain name is not lowercase ASCII".into());
        }
        if self.identifier.contains('%') && !uppercase_percent_hex(&self.identifier) {
            issues.push("percent-escapes are not uppercase hexadecimal".into());
        }
        issues
    }
}

impl fmt::Display for Tei {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.original)
    }
}

impl Serialize for Tei {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.original)
    }
}

impl<'de> Deserialize<'de> for Tei {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

fn validate_domain(domain: &str) -> Result<(), TeiError> {
    if domain.len() > 253
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains(':')
        || domain.contains('@')
    {
        return Err(TeiError::Domain);
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.iter().any(|label| !valid_dns_label(label)) {
        return Err(TeiError::Domain);
    }
    Ok(())
}

fn valid_dns_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    if bytes.is_empty() || bytes.len() > 63 {
        return false;
    }
    if bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

fn validate_type(tei_type: &str) -> Result<(), TeiError> {
    let mut chars = tei_type.chars();
    let Some(first) = chars.next() else {
        return Err(TeiError::Type);
    };
    if !first.is_ascii_lowercase() {
        return Err(TeiError::Type);
    }
    if chars.any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit()) {
        return Err(TeiError::Type);
    }
    Ok(())
}

fn validate_identifier_segment(identifier: &str) -> Result<(), TeiError> {
    let bytes = identifier.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' {
            return Err(TeiError::Identifier);
        }
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(TeiError::PercentEncoding);
            }
            // Compare bytes, not a `str` slice: a multi-byte character after
            // `%` would put `index + 3` inside a character and panic.
            let (high, low) = (bytes[index + 1], bytes[index + 2]);
            if !high.is_ascii_hexdigit() || !low.is_ascii_hexdigit() {
                return Err(TeiError::PercentEncoding);
            }
            if high == b'2' && low.eq_ignore_ascii_case(&b'f') {
                return Err(TeiError::Identifier);
            }
            index += 3;
            continue;
        }
        if bytes[index].is_ascii_control() || bytes[index] == b' ' {
            return Err(TeiError::Identifier);
        }
        index += 1;
    }
    Ok(())
}

fn uppercase_percent_hex(identifier: &str) -> bool {
    let bytes = identifier.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if bytes[index + 1..index + 3]
                .iter()
                .any(u8::is_ascii_lowercase)
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn validate_purl(identifier: &str) -> Result<(), TeiError> {
    if identifier.contains('=')
        || !identifier
            .bytes()
            .all(|b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_'))
    {
        return Err(registry("purl", "must be unpadded Base64URL"));
    }
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(identifier)
        .map_err(|_| registry("purl", "is not valid Base64URL"))?;
    let text =
        std::str::from_utf8(&decoded).map_err(|_| registry("purl", "does not decode to UTF-8"))?;
    if !text.starts_with("pkg:") || text.is_empty() {
        return Err(registry("purl", "does not decode to a Package URL"));
    }
    Ok(())
}

fn validate_hash(identifier: &str) -> Result<(), TeiError> {
    let Some((alg, hex_value)) = identifier.split_once(':') else {
        return Err(registry("hash", "must be `<hashtype>:<hex>`"));
    };
    let algorithm = ChecksumAlgorithm::from_wire(alg);
    let Some(len) = algorithm.digest_len() else {
        return Err(registry(
            "hash",
            "hashtype must be a checksum-type value such as SHA-256",
        ));
    };
    if hex_value.len() != len * 2
        || !hex_value.bytes().all(|b| b.is_ascii_hexdigit())
        || hex_value.bytes().any(|b| b.is_ascii_uppercase())
    {
        return Err(registry(
            "hash",
            "digest must be lowercase hexadecimal of the algorithm length",
        ));
    }
    Ok(())
}

fn validate_uuid_id(identifier: &str) -> Result<(), TeiError> {
    TeaUuid::parse(identifier)
        .map(|_| ())
        .map_err(|_| registry("uuid", "must be the lowercase hyphenated RFC 9562 form"))
}

fn validate_eanupc(identifier: &str) -> Result<(), TeiError> {
    if !matches!(identifier.len(), 8 | 12 | 13) {
        return Err(registry(
            "eanupc",
            "must be 8, 12, or 13 digits with its check digit",
        ));
    }
    gs1(identifier, "eanupc")
}

fn validate_gtin(identifier: &str) -> Result<(), TeiError> {
    if !matches!(identifier.len(), 8 | 12 | 13 | 14) {
        return Err(registry(
            "gtin",
            "must be a GTIN-8, GTIN-12, GTIN-13, or GTIN-14",
        ));
    }
    gs1(identifier, "gtin")
}

fn validate_asin(identifier: &str) -> Result<(), TeiError> {
    if identifier.len() != 10
        || !identifier
            .bytes()
            .all(|b| matches!(b, b'A'..=b'Z' | b'0'..=b'9'))
    {
        return Err(registry("asin", "must be 10 uppercase letters or digits"));
    }
    Ok(())
}

fn validate_udi(identifier: &str) -> Result<(), TeiError> {
    if identifier.is_empty() {
        return Err(registry("udi", "device identifier must not be empty"));
    }
    if identifier.starts_with("(01)") || identifier.starts_with('+') {
        return Err(registry(
            "udi",
            "must not include an application identifier or data identifier prefix",
        ));
    }
    Ok(())
}

fn gs1(identifier: &str, tei_type: &str) -> Result<(), TeiError> {
    if !identifier.bytes().all(|b| b.is_ascii_digit()) {
        return Err(registry(tei_type, "must be digits"));
    }
    let digits: Vec<u32> = identifier.bytes().map(|b| u32::from(b - b'0')).collect();
    let check = digits[digits.len() - 1];
    let mut sum = 0u32;
    for (index, digit) in digits[..digits.len() - 1].iter().rev().enumerate() {
        let weight = if index % 2 == 0 { 3 } else { 1 };
        sum += digit * weight;
    }
    let calculated = (10 - (sum % 10)) % 10;
    if calculated != check {
        return Err(registry(tei_type, "check digit is not valid"));
    }
    Ok(())
}

fn registry(tei_type: &str, reason: &str) -> TeiError {
    TeiError::Registry {
        tei_type: tei_type.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Percent-encode a query component using uppercase hex, leaving unreserved characters.
#[must_use]
pub fn encode_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn non_ascii_after_percent_is_an_error_not_a_panic() {
        for text in [
            "tei://example.com/uuid/%a\u{e9}",
            "tei://example.com/purl/x%a\u{e9}",
            "tei://example.com/purl/%\u{e9}a",
            "tei://example.com/hash/%4\u{1f600}",
            "tei://example.com/x/%\u{e9}",
        ] {
            assert!(Tei::parse(text).is_err(), "{text}");
        }
        // A valid escape followed by non-ASCII text is still checked byte-wise.
        let tei = Tei::parse("tei://example.com/x/%41\u{e9}").unwrap();
        assert!(tei.publication_issues().is_empty());
        let lower = Tei::parse("tei://example.com/x/%4a\u{e9}").unwrap();
        assert_eq!(lower.publication_issues().len(), 1);
    }

    use super::*;

    const UUID_TEI: &str = "tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1";
    const PURL_TEI: &str =
        "tei://example.com/purl/cGtnOm1hdmVuL2NvbW1vbnMtaW8vY29tbW9ucy1pb0AyLjIyLjA";
    const HASH_TEI: &str = "tei://example.com/hash/SHA-256:fd44efd601f651c8865acf0dfeacb0df19a2b50ec69ead0262096fd2f67197b9";

    #[test]
    fn parses_registry_examples() {
        for tei in [
            UUID_TEI,
            PURL_TEI,
            HASH_TEI,
            "tei://example.com/eanupc/1234567890128",
            "tei://example.com/gtin/0234567890129",
            "tei://example.com/asin/B07FZ8S74R",
            "tei://example.com/udi/00123456789012",
        ] {
            let parsed = Tei::parse(tei).unwrap();
            parsed.validate_registry().unwrap();
            assert_eq!(parsed.as_str(), tei);
            assert_eq!(parsed.to_string(), tei);
        }
    }

    #[test]
    fn does_not_treat_case_variants_as_equal() {
        let lower = Tei::parse(UUID_TEI).unwrap();
        let upper_domain =
            Tei::parse("tei://Example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
        assert_ne!(lower, upper_domain);
        assert!(!upper_domain.publication_issues().is_empty());
        assert!(Tei::parse("TEI://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").is_err());
        assert!(Tei::parse("tei://example.com/UUID/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").is_err());
    }

    #[test]
    fn rejects_slash_and_port() {
        assert!(Tei::parse("tei://example.com/uuid/a/b").is_err());
        assert!(Tei::parse("tei://example.com/uuid/a%2Fb").is_err());
        assert!(Tei::parse("tei://example.com/uuid/a%2fb").is_err());
        assert!(
            Tei::parse("tei://example.com:443/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").is_err()
        );
    }

    #[test]
    fn unknown_type_parses_without_registry_rules() {
        let tei = Tei::parse("tei://example.com/sku/ABC-1").unwrap();
        assert!(tei.validate_registry().is_ok());
    }

    #[test]
    fn padded_purl_is_rejected() {
        let tei =
            Tei::parse("tei://example.com/purl/cGtnOnB5cGkvY3ljbG9uZWR4LXB5dGhvbi1saWI=").unwrap();
        assert!(tei.validate_registry().is_err());
    }

    #[test]
    fn query_encoding_matches_openapi_example_shape() {
        let encoded = encode_query_component(
            "tei://products.example.com/uuid/47f51950-b0e2-11f1-adc2-1a52914d44b2",
        );
        assert_eq!(
            encoded,
            "tei%3A%2F%2Fproducts.example.com%2Fuuid%2F47f51950-b0e2-11f1-adc2-1a52914d44b2"
        );
    }
}
