//! Protocol errors for parsing and validation.
//!
//! These errors describe malformed or non-conforming TEA data. They are not
//! transport failures.

use thiserror::Error;

/// Escape control characters so server-supplied text is safe to print.
///
/// C0 controls (including ESC, BEL, and CR), DEL, C1 controls (including the
/// single-character CSI `U+009B` and OSC `U+009D`), and the bidirectional
/// override/isolate characters (`U+202A..=U+202E`, `U+2066..=U+2069`) become
/// `\uXXXX`. Tab and newline are kept. Other text is returned unchanged. A
/// backslash sequence that the server itself sent cannot be told apart from an
/// escape.
///
/// `libtea-client` applies this to server text it puts into
/// [`ModelError`] and its own error type, and `libtea-cli` applies it to
/// everything it prints.
#[must_use]
pub fn escape_control_chars(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let escape = match c {
            '\t' | '\n' => false,
            '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' => true,
            c => c.is_control(),
        };
        if escape {
            out.push_str(&format!("\\u{:04x}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

/// Failure while parsing or validating a Transparency Exchange Identifier.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TeiError {
    /// The input was empty.
    #[error("TEI is empty")]
    Empty,
    /// The input used the legacy `urn:tei:` form, which this crate does not parse.
    #[error(
        "the legacy `urn:tei:` form is not supported; use `tei://<domain-name>/<type>/<unique-identifier>`"
    )]
    LegacyUrn,
    /// The scheme was not the exact lowercase string `tei`.
    #[error("TEI scheme must be the lowercase string `tei`")]
    Scheme,
    /// The TEI did not have the form `tei://domain/type/identifier`.
    #[error("TEI must be `tei://<domain-name>/<type>/<unique-identifier>`")]
    Structure,
    /// A domain name was missing or not an ASCII DNS name.
    #[error("TEI domain name is not an ASCII DNS name without a port")]
    Domain,
    /// The type segment was not lowercase ASCII starting with a letter.
    #[error("TEI type must be lowercase ASCII letters and digits, starting with a letter")]
    Type,
    /// The unique-identifier segment was empty or contained a slash.
    #[error("TEI unique identifier must be one path segment and must not contain `/`")]
    Identifier,
    /// A percent-escape was truncated or not hexadecimal.
    #[error("TEI unique identifier contains an invalid percent-escape")]
    PercentEncoding,
    /// The identifier failed the registered type's syntax.
    #[error("TEI identifier is not valid for type `{tei_type}`: {reason}")]
    #[non_exhaustive]
    Registry {
        /// Registered TEI type that rejected the identifier.
        tei_type: String,
        /// Why the identifier failed that type's syntax.
        reason: String,
    },
}

/// Failure while parsing a TEA model, discovery document, or version.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelError {
    /// JSON was not valid.
    #[error("JSON parse error: {}", escape_control_chars(.0))]
    Json(String),
    /// A field violated a normative constraint.
    ///
    /// The message can quote server text; `Display` escapes control characters
    /// ([`escape_control_chars`]).
    #[error("{}", escape_control_chars(.0))]
    Invalid(String),
}

impl From<serde_json::Error> for ModelError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

/// Failure of destination policy checks that do not contact the network.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PolicyError {
    /// The URL used a scheme other than `https`, or `http` outside an explicit exception.
    #[error("URL scheme is not allowed: {0}")]
    Scheme(String),
    /// The URL contained userinfo.
    #[error("URL must not contain userinfo")]
    Userinfo,
    /// The URL contained a query or fragment where a base URL forbids them.
    #[error("URL must not contain a query or fragment")]
    QueryOrFragment,
    /// The host was missing.
    #[error("URL is missing a host")]
    Host,
    /// A trailing slash was present on an API base.
    #[error("API base URL must not have a trailing slash")]
    TrailingSlash,
    /// The resolved or literal address is not an acceptable destination.
    #[error("destination address is not allowed: {0}")]
    Destination(String),
    /// The hostname is a well-known metadata or local name.
    #[error("hostname is not allowed: {0}")]
    Hostname(String),
}
