//! Package URL (ECMA-427) parsing and canonical form, for comparison only.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! TEA asks a client to use a TEI "exactly as received" and to send the `purl`
//! query parameter as the caller's "logical PURL string". So nothing in this
//! SDK rewrites a PURL it sends, stores, locks or prints as the server's. This
//! module exists so a caller can ask whether two PURLs name the same package:
//! [`Purl`] keeps the text it was parsed from ([`Purl::as_str`]), and the
//! canonical form is a separate type, [`CanonicalPurl`], that cannot be passed
//! where a `Purl` is expected.
//!
//! The rules ECMA-427 gives for every type are always applied
//! ([`PurlRules::Generic`]): lowercase scheme, type and qualifier keys;
//! percent-encoding normalized; empty qualifier values dropped and keys sorted;
//! empty, `.` and `..` subpath segments dropped. Type-specific rules are
//! applied only for the types [`PurlRules`] names (so far `pypi`); for every
//! other type, two spellings that a type-aware tool would treat as the same
//! package may compare unequal. [`CanonicalPurl::rules`] says which applied.
//!
//! See `docs/design/purl.md` and the test-suite data under `spec/purl/`.

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

/// Inputs longer than this, or whose canonical form would be, are rejected
/// with [`PurlError::TooLong`].
pub const MAX_PURL_BYTES: usize = 4096;

/// Why a string is not a Package URL.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PurlError {
    /// The text, or its canonical form, is longer than [`MAX_PURL_BYTES`].
    #[error("Package URL is longer than {MAX_PURL_BYTES} bytes")]
    TooLong,
    /// The scheme is missing or is not `pkg`.
    #[error("Package URL must start with `pkg:`")]
    Scheme,
    /// The type is missing, does not start with a letter, or has characters
    /// other than ASCII letters, digits, `.` and `-`.
    #[error("Package URL type is missing or invalid")]
    Type,
    /// The name is missing.
    #[error("Package URL name is missing")]
    Name,
    /// A namespace segment decodes to text containing `/`.
    #[error("Package URL namespace segment contains a slash")]
    Namespace,
    /// A qualifier key is empty, has characters other than ASCII letters,
    /// digits, `.`, `-` and `_`, or does not start with a letter.
    #[error("Package URL qualifier key is invalid")]
    QualifierKey,
    /// A qualifier key appears twice.
    #[error("Package URL qualifier key appears more than once")]
    DuplicateQualifier,
    /// A subpath segment decodes to text containing `/`.
    #[error("Package URL subpath segment contains a slash")]
    Subpath,
    /// A `%` not followed by two hex digits, or decoded bytes that are not UTF-8.
    #[error("Package URL has an invalid percent-encoding")]
    Encoding,
    /// The identifier of a `purl` TEI is not unpadded Base64URL of UTF-8 text.
    #[error("TEI purl identifier is not unpadded Base64URL of UTF-8 text")]
    TeiIdentifier,
}

/// Which canonicalization rules produced a [`CanonicalPurl`].
///
/// **Experimental:** may change in a 0.x minor release. A type that gains
/// rules gets a new variant, so its canonical form changes in that release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PurlRules {
    /// The rules ECMA-427 gives for every type, and nothing type-specific.
    Generic,
    /// The generic rules, plus those of the purl-spec `pypi` type definition:
    /// the name is lowercased and each run of `-`, `_` and `.` becomes one
    /// `-` (PEP 503), and the version, which the definition marks
    /// case-insensitive, is lowercased. Only ASCII letters are lowercased;
    /// valid PyPI names and versions are ASCII.
    Pypi,
}

impl PurlRules {
    /// The rules for a lowercased type.
    fn for_type(package_type: &str) -> Self {
        match package_type {
            "pypi" => Self::Pypi,
            _ => Self::Generic,
        }
    }
}

/// A parsed Package URL. Holds the original text and never rewrites it.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// Equality compares the original text and components exactly. To ask
/// whether two spellings name the same package, compare
/// [`Purl::canonical`] values or use [`same_package_url`].
///
/// ```
/// use libtea_core::purl::{same_package_url, Purl};
///
/// let a = Purl::parse("PKG:Maven/org.apache/commons-io@2.11.0?Type=jar&classifier=").unwrap();
/// let b = Purl::parse("pkg:maven/org.apache/commons-io@2.11.0?type=jar").unwrap();
/// assert_eq!(a.as_str(), "PKG:Maven/org.apache/commons-io@2.11.0?Type=jar&classifier=");
/// assert_eq!(a.canonical().as_str(), "pkg:maven/org.apache/commons-io@2.11.0?type=jar");
/// assert!(same_package_url(&a, &b));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Purl {
    original: String,
    package_type: String,
    namespace: Vec<String>,
    name: String,
    version: Option<String>,
    qualifiers: BTreeMap<String, String>,
    subpath: Vec<String>,
}

/// A PURL in canonical form under [`Self::rules`]. Comparable and hashable.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// It is computed from a [`Purl`] and cannot be turned back into one without
/// parsing, so it cannot be sent by mistake where the original was meant.
///
/// Equality, hashing and ordering compare the text. The rules follow from the
/// type in it, so equal texts always have equal rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalPurl {
    text: String,
    rules: PurlRules,
}

impl CanonicalPurl {
    /// The canonical text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The rules that were applied: [`PurlRules::Generic`], or the
    /// type-specific variant for a type that has one.
    #[must_use]
    pub fn rules(&self) -> PurlRules {
        self.rules
    }
}

impl fmt::Display for CanonicalPurl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Whether two PURLs have the same canonical form ([`Purl::canonical`]).
#[must_use]
pub fn same_package_url(a: &Purl, b: &Purl) -> bool {
    a.canonical() == b.canonical()
}

impl Purl {
    /// Parse a Package URL as ECMA-427 describes, right to left.
    ///
    /// Accepts what the specification says a parser should accept, such as
    /// `pkg://` and redundant slashes, and keeps `text` unchanged for
    /// [`Self::as_str`].
    ///
    /// # Errors
    ///
    /// Returns [`PurlError`] when a required component is missing, a type or
    /// qualifier key has characters ECMA-427 does not allow, a qualifier key
    /// repeats, a decoded namespace or subpath segment contains `/`, the
    /// percent-encoding is invalid, or the text or its canonical form is
    /// longer than [`MAX_PURL_BYTES`].
    pub fn parse(text: &str) -> Result<Self, PurlError> {
        if text.len() > MAX_PURL_BYTES {
            return Err(PurlError::TooLong);
        }
        let (remainder, subpath) = match text.rsplit_once('#') {
            Some((left, right)) => (left, parse_subpath(right)?),
            None => (text, Vec::new()),
        };
        let (remainder, qualifiers) = match remainder.rsplit_once('?') {
            Some((left, right)) => (left, parse_qualifiers(right)?),
            None => (remainder, BTreeMap::new()),
        };
        let (scheme, remainder) = remainder.split_once(':').ok_or(PurlError::Scheme)?;
        if !scheme.eq_ignore_ascii_case("pkg") {
            return Err(PurlError::Scheme);
        }
        let remainder = remainder.trim_start_matches('/');
        let (package_type, remainder) = remainder.split_once('/').ok_or(PurlError::Name)?;
        let package_type = package_type.to_ascii_lowercase();
        if !valid_type(&package_type) {
            return Err(PurlError::Type);
        }
        // The version separator is the last '@' of the last path segment, so an
        // unencoded '@' that starts a namespace segment (`pkg:npm/@babel/core`)
        // is not taken for one. A version's own '/' is percent-encoded.
        let last_segment = remainder.rfind('/').map_or(0, |index| index + 1);
        let (remainder, version) = match remainder[last_segment..].rfind('@') {
            Some(at) => {
                let at = last_segment + at;
                let version = percent_decode(&remainder[at + 1..])?;
                (&remainder[..at], (!version.is_empty()).then_some(version))
            }
            None => (remainder, None),
        };
        let remainder = remainder.trim_end_matches('/');
        let (remainder, name) = match remainder.rsplit_once('/') {
            Some((left, right)) => (left, right),
            None => ("", remainder),
        };
        let name = percent_decode(name)?;
        if name.is_empty() {
            return Err(PurlError::Name);
        }
        let mut namespace = Vec::new();
        for segment in remainder.split('/').filter(|segment| !segment.is_empty()) {
            let segment = percent_decode(segment)?;
            if segment.contains('/') {
                return Err(PurlError::Namespace);
            }
            namespace.push(segment);
        }
        let purl = Self {
            original: text.to_owned(),
            package_type,
            namespace,
            name,
            version,
            qualifiers,
            subpath,
        };
        // Percent-encoding can triple a component's length. Refusing here keeps
        // the promise that every canonical form parses again.
        if purl.canonical().text.len() > MAX_PURL_BYTES {
            return Err(PurlError::TooLong);
        }
        Ok(purl)
    }

    /// Exactly the text that was parsed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// The type, lowercased.
    #[must_use]
    pub fn package_type(&self) -> &str {
        &self.package_type
    }

    /// The decoded namespace segments joined with `/`, or `None` when there
    /// are none. No type-specific normalization is applied (see
    /// [`Self::canonical`]).
    #[must_use]
    pub fn namespace(&self) -> Option<String> {
        (!self.namespace.is_empty()).then(|| self.namespace.join("/"))
    }

    /// The decoded name, as written. No type-specific normalization is
    /// applied (see [`Self::canonical`]).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The decoded version, as written, or `None` when absent or empty.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Decoded qualifiers with non-empty values, keys lowercased, in key order.
    pub fn qualifiers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.qualifiers
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// The decoded value of one qualifier; `key` is matched in lowercase.
    #[must_use]
    pub fn qualifier(&self, key: &str) -> Option<&str> {
        self.qualifiers
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// The decoded subpath segments joined with `/`, without empty, `.` and
    /// `..` segments, or `None` when none remain.
    #[must_use]
    pub fn subpath(&self) -> Option<String> {
        (!self.subpath.is_empty()).then(|| self.subpath.join("/"))
    }

    /// The canonical form: the generic rules, plus the type's own rules when
    /// [`PurlRules`] has them ([`CanonicalPurl::rules`] says which).
    ///
    /// ```
    /// use libtea_core::purl::{Purl, PurlRules};
    ///
    /// let purl = Purl::parse("pkg:PyPI/Django_Allauth@0.61.1RC1").unwrap();
    /// assert_eq!(purl.name(), "Django_Allauth");
    /// let canonical = purl.canonical();
    /// assert_eq!(canonical.as_str(), "pkg:pypi/django-allauth@0.61.1rc1");
    /// assert_eq!(canonical.rules(), PurlRules::Pypi);
    /// ```
    #[must_use]
    pub fn canonical(&self) -> CanonicalPurl {
        let rules = PurlRules::for_type(&self.package_type);
        let mut out = String::with_capacity(self.original.len());
        out.push_str("pkg:");
        out.push_str(&self.package_type);
        out.push('/');
        for segment in &self.namespace {
            percent_encode_into(segment, &mut out);
            out.push('/');
        }
        match rules {
            PurlRules::Pypi => percent_encode_into(&pypi_name(&self.name), &mut out),
            PurlRules::Generic => percent_encode_into(&self.name, &mut out),
        }
        if let Some(version) = &self.version {
            out.push('@');
            match rules {
                PurlRules::Pypi => percent_encode_into(&version.to_ascii_lowercase(), &mut out),
                PurlRules::Generic => percent_encode_into(version, &mut out),
            }
        }
        for (index, (key, value)) in self.qualifiers.iter().enumerate() {
            out.push(if index == 0 { '?' } else { '&' });
            out.push_str(key);
            out.push('=');
            percent_encode_into(value, &mut out);
        }
        for (index, segment) in self.subpath.iter().enumerate() {
            out.push(if index == 0 { '#' } else { '/' });
            percent_encode_into(segment, &mut out);
        }
        CanonicalPurl { text: out, rules }
    }
}

/// PEP 503: lowercase, and each run of `-`, `_` and `.` becomes one `-`.
fn pypi_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut in_run = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !in_run {
                out.push('-');
            }
            in_run = true;
        } else {
            out.push(c.to_ascii_lowercase());
            in_run = false;
        }
    }
    out
}

/// ECMA-427 5.6.2: ASCII letters, digits, `.` and `-`, starting with a letter.
fn valid_type(package_type: &str) -> bool {
    let mut chars = package_type.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

/// ECMA-427 5.6.6, keys already lowercased: letters, digits, `.`, `-` and `_`,
/// starting with a letter.
fn valid_qualifier_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

fn parse_qualifiers(text: &str) -> Result<BTreeMap<String, String>, PurlError> {
    let mut qualifiers = BTreeMap::new();
    for pair in text.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = key.to_ascii_lowercase();
        if !valid_qualifier_key(&key) {
            return Err(PurlError::QualifierKey);
        }
        let value = percent_decode(value)?;
        if value.is_empty() {
            continue;
        }
        if qualifiers.insert(key, value).is_some() {
            return Err(PurlError::DuplicateQualifier);
        }
    }
    Ok(qualifiers)
}

fn parse_subpath(text: &str) -> Result<Vec<String>, PurlError> {
    let mut segments = Vec::new();
    for segment in text.split('/') {
        let segment = percent_decode(segment)?;
        if segment.is_empty() || segment == "." || segment == ".." {
            continue;
        }
        if segment.contains('/') {
            return Err(PurlError::Subpath);
        }
        segments.push(segment);
    }
    Ok(segments)
}

fn percent_decode(text: &str) -> Result<String, PurlError> {
    if !text.contains('%') {
        return Ok(text.to_owned());
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).and_then(|b| hex_value(*b));
            let low = bytes.get(index + 2).and_then(|b| hex_value(*b));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(PurlError::Encoding);
            };
            out.push(high << 4 | low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| PurlError::Encoding)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// ECMA-427 5.4: alphanumerics, `.-_~` and `:` stay; every other UTF-8 octet
/// becomes `%XX` with uppercase hex.
fn percent_encode_into(text: &str, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'~' | b':') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_text_is_kept_and_canonical_is_separate() {
        let text = "pkg://NPM//%40angular/animation@12.3.1?B=2&a=1#/x/./y/";
        let purl = Purl::parse(text).unwrap();
        assert_eq!(purl.as_str(), text);
        assert_eq!(purl.package_type(), "npm");
        assert_eq!(purl.namespace().as_deref(), Some("@angular"));
        assert_eq!(purl.name(), "animation");
        assert_eq!(purl.version(), Some("12.3.1"));
        assert_eq!(purl.qualifier("A"), Some("1"));
        assert_eq!(purl.subpath().as_deref(), Some("x/y"));
        assert_eq!(
            purl.canonical().as_str(),
            "pkg:npm/%40angular/animation@12.3.1?a=1&b=2#x/y"
        );
        assert_eq!(purl.canonical().rules(), PurlRules::Generic);
    }

    #[test]
    fn an_unencoded_scope_is_not_a_version() {
        let purl = Purl::parse("pkg:npm/@babel/core#lib").unwrap();
        assert_eq!(purl.namespace().as_deref(), Some("@babel"));
        assert_eq!(purl.name(), "core");
        assert_eq!(purl.version(), None);
        assert_eq!(purl.canonical().as_str(), "pkg:npm/%40babel/core#lib");
        let versioned = Purl::parse("pkg:npm/@babel/core@7.0.0").unwrap();
        assert_eq!(versioned.version(), Some("7.0.0"));
    }

    #[test]
    fn canonical_is_idempotent_on_its_own_output() {
        for text in [
            "pkg:generic/a%20b@1.0%2B2?checksum=sha1:ab,sha256:cd",
            "pkg:deb/debian/curl@7.50.3-1?arch=i386&distro=jessie",
            "pkg:golang/github.com/gorilla/context@234fd47e07d1004f0aed9c#api",
            "pkg:generic/%E2%82%AC@%C3%A9",
        ] {
            let once = Purl::parse(text).unwrap().canonical();
            let twice = Purl::parse(once.as_str()).unwrap().canonical();
            assert_eq!(once, twice, "{text}");
        }
    }

    #[test]
    fn rejected_inputs() {
        let cases: &[(&str, PurlError)] = &[
            ("EnterpriseLibrary.Common@6.0.1304", PurlError::Scheme),
            ("http:npm/a", PurlError::Scheme),
            ("pkg:npm", PurlError::Name),
            ("pkg:npm/", PurlError::Name),
            ("pkg:npm/@1.0", PurlError::Name),
            ("pkg:/a", PurlError::Name),
            ("pkg:9pm/a", PurlError::Type),
            ("pkg:n+m/a", PurlError::Type),
            ("pkg:n%70m/a", PurlError::Type),
            ("pkg:npm/a?1key=v", PurlError::QualifierKey),
            ("pkg:npm/a?k%65y=v", PurlError::QualifierKey),
            ("pkg:npm/a?=v", PurlError::QualifierKey),
            ("pkg:npm/a?k=1&K=2", PurlError::DuplicateQualifier),
            ("pkg:npm/a%2Fb/c", PurlError::Namespace),
            ("pkg:npm/a#b%2Fc", PurlError::Subpath),
            ("pkg:npm/a%zz", PurlError::Encoding),
            ("pkg:npm/a%c3", PurlError::Encoding),
            ("pkg:npm/a@1%", PurlError::Encoding),
        ];
        for (text, error) in cases {
            assert_eq!(Purl::parse(text).as_ref().err(), Some(error), "{text}");
        }
        let long = format!("pkg:generic/{}", "a".repeat(MAX_PURL_BYTES));
        assert_eq!(Purl::parse(&long), Err(PurlError::TooLong));
        // Short enough as text, too long once each space becomes `%20`.
        let spaces = format!("pkg:generic/{}", " ".repeat(2000));
        assert!(spaces.len() <= MAX_PURL_BYTES);
        assert_eq!(Purl::parse(&spaces), Err(PurlError::TooLong));
    }

    #[test]
    fn empty_qualifier_values_are_dropped_and_empty_version_is_none() {
        let purl = Purl::parse("pkg:generic/a@?x=&y").unwrap();
        assert_eq!(purl.version(), None);
        assert_eq!(purl.qualifiers().count(), 0);
        assert_eq!(purl.canonical().as_str(), "pkg:generic/a");
    }

    #[test]
    fn names_are_not_case_folded_without_a_type_rule() {
        // npm and maven names are case-sensitive in their type definitions.
        for (a, b) in [
            ("pkg:npm/Lodash@4.0.0", "pkg:npm/lodash@4.0.0"),
            (
                "pkg:maven/org.Apache/Commons@1",
                "pkg:maven/org.apache/commons@1",
            ),
            ("pkg:generic/a_b@1", "pkg:generic/a-b@1"),
        ] {
            let (a, b) = (Purl::parse(a).unwrap(), Purl::parse(b).unwrap());
            assert!(!same_package_url(&a, &b));
            assert_eq!(a.canonical().rules(), PurlRules::Generic);
        }
    }

    #[test]
    fn pypi_names_and_versions_follow_the_type_definition() {
        for (input, canonical) in [
            ("pkg:pypi/Django@1.11.1", "pkg:pypi/django@1.11.1"),
            ("pkg:pypi/friendly.bard@1.0", "pkg:pypi/friendly-bard@1.0"),
            ("pkg:pypi/friendly--bard", "pkg:pypi/friendly-bard"),
            (
                "pkg:pypi/My.PyPI_.--._Package@1.11.1.DEV1",
                "pkg:pypi/my-pypi-package@1.11.1.dev1",
            ),
            ("pkg:pypi/-a_", "pkg:pypi/-a-"),
            ("pkg:pypi/a%2Eb", "pkg:pypi/a-b"),
            // Qualifier values and the subpath keep their case.
            (
                "pkg:pypi/Django@1.0?file_name=Django-1.0.tar.gz#Docs",
                "pkg:pypi/django@1.0?file_name=Django-1.0.tar.gz#Docs",
            ),
        ] {
            let purl = Purl::parse(input).unwrap();
            assert_eq!(purl.as_str(), input);
            assert_eq!(purl.canonical().as_str(), canonical, "{input}");
            assert_eq!(purl.canonical().rules(), PurlRules::Pypi);
            let again = Purl::parse(canonical).unwrap().canonical();
            assert_eq!(again.as_str(), canonical, "idempotent: {input}");
        }
        let a = Purl::parse("pkg:pypi/Zope.Interface@5.0").unwrap();
        let b = Purl::parse("pkg:pypi/zope-interface@5.0").unwrap();
        assert!(same_package_url(&a, &b));
        // The accessors keep what was written.
        assert_eq!(a.name(), "Zope.Interface");
    }
}
