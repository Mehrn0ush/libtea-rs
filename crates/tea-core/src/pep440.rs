//! Python package versions (PEP 440, the "Version specifiers" specification
//! of the Python Packaging User Guide), parsed and ordered.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! [`Pep440Version::parse`] accepts what the specification's "Appendix:
//! Parsing version strings with regular expressions" accepts (the
//! `VERSION_PATTERN` used by the `packaging` library), case-insensitively:
//! an optional `v`, an epoch (`1!`), a release (`1.2.3`), a pre-release
//! (`a`, `b`, `rc` and their alternative spellings), a post-release (`.post1`,
//! `-1`), a developmental release (`.dev1`) and a local label (`+ubuntu.1`).
//! Surrounding whitespace is **not** stripped: a version with whitespace is
//! an error, as it is inside a `vers` range.
//!
//! Ordering is the specification's: trailing zeros of the release do not
//! count (`1.0 == 1.0.0`), `1.0.dev1 < 1.0a1 < 1.0 < 1.0.post1`, and a local
//! label sorts after the same version without one. Equality and hashing
//! follow the ordering, so `1.0` and `1.0.0` are equal; [`Pep440Version::as_str`]
//! still returns what was written, and `Display` prints the normalized form.
//!
//! This is ordering only, as a `vers:pypi` range needs. The specifier
//! semantics of PEP 440 itself (`~=`, `==1.*`, the exclusion of pre-releases
//! by `<`) are not implemented.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use thiserror::Error;

/// Inputs longer than this are rejected with [`Pep440Error::TooLong`].
pub const MAX_PEP440_BYTES: usize = 1024;

/// Why a string is not a PEP 440 version.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Pep440Error {
    /// Longer than [`MAX_PEP440_BYTES`].
    #[error("version is longer than {MAX_PEP440_BYTES} bytes")]
    TooLong,
    /// Not a version in the form the specification accepts.
    #[error("not a PEP 440 version")]
    Invalid,
    /// A number does not fit in 64 bits.
    #[error("a number in the version does not fit in 64 bits")]
    NumberTooLarge,
}

/// The kind of a pre-release, in ascending order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PreRelease {
    /// `a`, also spelled `alpha`.
    Alpha,
    /// `b`, also spelled `beta`.
    Beta,
    /// `rc`, also spelled `c`, `pre` and `preview`.
    ReleaseCandidate,
}

impl PreRelease {
    /// The normalized spelling.
    fn as_str(self) -> &'static str {
        match self {
            Self::Alpha => "a",
            Self::Beta => "b",
            Self::ReleaseCandidate => "rc",
        }
    }
}

/// One dot-separated part of a local version label.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LocalSegment {
    Number(u64),
    Text(String),
}

impl Ord for LocalSegment {
    /// The specification: numeric parts sort after alphanumeric ones; numbers
    /// compare as numbers, text lexicographically (already lowercase).
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            (Self::Text(_), Self::Number(_)) => Ordering::Less,
            (Self::Number(_), Self::Text(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for LocalSegment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A parsed PEP 440 version.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// ```
/// use libtea_core::pep440::Pep440Version;
///
/// let v = |s: &str| Pep440Version::parse(s).unwrap();
/// assert!(v("1.0.dev1") < v("1.0a1"));
/// assert!(v("1.0a1") < v("1.0"));
/// assert!(v("1.0") < v("1.0.post1"));
/// assert!(v("1.0") < v("1.0+local.1"));
/// assert_eq!(v("1.0"), v("1.0.0"));
/// assert_eq!(v("V1.0-ALPHA.2").to_string(), "1.0a2");
/// assert_eq!(v("V1.0-ALPHA.2").as_str(), "V1.0-ALPHA.2");
/// ```
#[derive(Debug, Clone)]
pub struct Pep440Version {
    original: String,
    epoch: u64,
    release: Vec<u64>,
    pre: Option<(PreRelease, u64)>,
    post: Option<u64>,
    dev: Option<u64>,
    local: Vec<LocalSegment>,
}

impl Pep440Version {
    /// Parse a version as the specification's regular expression does.
    ///
    /// # Errors
    ///
    /// [`Pep440Error::Invalid`] for text that is not a version (including
    /// any whitespace or non-ASCII character), [`Pep440Error::NumberTooLarge`]
    /// for a number above `u64::MAX`, and [`Pep440Error::TooLong`] for text
    /// longer than [`MAX_PEP440_BYTES`].
    pub fn parse(text: &str) -> Result<Self, Pep440Error> {
        if text.len() > MAX_PEP440_BYTES {
            return Err(Pep440Error::TooLong);
        }
        if !text.is_ascii() {
            return Err(Pep440Error::Invalid);
        }
        let lower = text.to_ascii_lowercase();
        let mut cursor = Cursor {
            rest: lower.as_str(),
        };
        cursor.eat("v");

        let first = cursor.number()?.ok_or(Pep440Error::Invalid)?;
        let (epoch, first) = if cursor.eat("!") {
            (first, cursor.number()?.ok_or(Pep440Error::Invalid)?)
        } else {
            (0, first)
        };
        let mut release = vec![first];
        while cursor.rest.starts_with('.')
            && cursor.rest[1..].starts_with(|c: char| c.is_ascii_digit())
        {
            cursor.eat(".");
            release.push(cursor.number()?.ok_or(Pep440Error::Invalid)?);
        }

        let pre = cursor.labelled(&[
            ("alpha", PreRelease::Alpha),
            ("a", PreRelease::Alpha),
            ("beta", PreRelease::Beta),
            ("b", PreRelease::Beta),
            ("preview", PreRelease::ReleaseCandidate),
            ("pre", PreRelease::ReleaseCandidate),
            ("c", PreRelease::ReleaseCandidate),
            ("rc", PreRelease::ReleaseCandidate),
        ])?;
        // `-N` is an implicit post-release; otherwise a spelled one.
        let post = match cursor.rest.strip_prefix('-') {
            Some(after) if after.starts_with(|c: char| c.is_ascii_digit()) => {
                cursor.eat("-");
                cursor.number()?
            }
            _ => cursor
                .labelled(&[("post", ()), ("rev", ()), ("r", ())])?
                .map(|((), n)| n),
        };
        let dev = cursor.labelled(&[("dev", ())])?.map(|((), n)| n);

        let mut local = Vec::new();
        if cursor.eat("+") {
            for part in cursor.rest.split(['.', '-', '_']) {
                if part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    return Err(Pep440Error::Invalid);
                }
                local.push(if part.bytes().all(|b| b.is_ascii_digit()) {
                    LocalSegment::Number(part.parse().map_err(|_| Pep440Error::NumberTooLarge)?)
                } else {
                    LocalSegment::Text(part.to_owned())
                });
            }
            cursor.rest = "";
        }
        if !cursor.rest.is_empty() {
            return Err(Pep440Error::Invalid);
        }
        Ok(Self {
            original: text.to_owned(),
            epoch,
            release,
            pre,
            post,
            dev,
            local,
        })
    }

    /// Exactly the text that was parsed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// Whether this is a pre-release or a developmental release.
    #[must_use]
    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some() || self.dev.is_some()
    }

    /// The release without trailing zeros, for ordering and hashing.
    fn trimmed_release(&self) -> &[u64] {
        let end = self
            .release
            .iter()
            .rposition(|n| *n != 0)
            .map_or(0, |index| index + 1);
        &self.release[..end]
    }

    /// The specification's sort key, after the epoch and release: a
    /// developmental release of a final version sorts before its
    /// pre-releases, no pre-release after them; no post-release before any;
    /// no developmental release after any.
    fn suffix_key(&self) -> SuffixKey {
        let pre = match (self.pre, self.post, self.dev) {
            (None, None, Some(_)) => (0, None),
            (Some(pre), _, _) => (1, Some(pre)),
            (None, _, _) => (2, None),
        };
        let dev = self.dev.map_or((1, 0), |n| (0, n));
        (pre, self.post, dev)
    }
}

impl Ord for Pep440Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.epoch
            .cmp(&other.epoch)
            .then_with(|| self.trimmed_release().cmp(other.trimmed_release()))
            .then_with(|| self.suffix_key().cmp(&other.suffix_key()))
            // No local label sorts before any; `Vec` ordering does that, as
            // an empty vector is less than any other.
            .then_with(|| self.local.cmp(&other.local))
    }
}

impl PartialOrd for Pep440Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Pep440Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Pep440Version {}

impl Hash for Pep440Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.epoch.hash(state);
        self.trimmed_release().hash(state);
        self.suffix_key().hash(state);
        self.local.hash(state);
    }
}

impl fmt::Display for Pep440Version {
    /// The normalized form: `[N!]N(.N)*[{a|b|rc}N][.postN][.devN][+local]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.epoch != 0 {
            write!(f, "{}!", self.epoch)?;
        }
        for (index, part) in self.release.iter().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            write!(f, "{part}")?;
        }
        if let Some((kind, n)) = self.pre {
            write!(f, "{}{n}", kind.as_str())?;
        }
        if let Some(n) = self.post {
            write!(f, ".post{n}")?;
        }
        if let Some(n) = self.dev {
            write!(f, ".dev{n}")?;
        }
        for (index, part) in self.local.iter().enumerate() {
            f.write_str(if index == 0 { "+" } else { "." })?;
            match part {
                LocalSegment::Number(n) => write!(f, "{n}")?,
                LocalSegment::Text(text) => f.write_str(text)?,
            }
        }
        Ok(())
    }
}

/// The pre-, post- and dev-release part of the sort key, in that order.
type SuffixKey = ((u8, Option<(PreRelease, u64)>), Option<u64>, (u8, u64));

/// The unparsed rest of a lowercased version.
struct Cursor<'a> {
    rest: &'a str,
}

impl Cursor<'_> {
    fn eat(&mut self, prefix: &str) -> bool {
        match self.rest.strip_prefix(prefix) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    /// A run of digits, or `None` when there is none.
    fn number(&mut self) -> Result<Option<u64>, Pep440Error> {
        let end = self
            .rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(self.rest.len());
        if end == 0 {
            return Ok(None);
        }
        let (digits, rest) = self.rest.split_at(end);
        self.rest = rest;
        digits
            .parse()
            .map(Some)
            .map_err(|_| Pep440Error::NumberTooLarge)
    }

    /// `[-_.]? label [-_.]? N?`, with the first label that matches; the
    /// number defaults to 0. Nothing is consumed when no label matches.
    fn labelled<T: Copy>(&mut self, labels: &[(&str, T)]) -> Result<Option<(T, u64)>, Pep440Error> {
        let after_separator = self.rest.strip_prefix(['-', '_', '.']).unwrap_or(self.rest);
        let Some((value, rest)) = labels.iter().find_map(|(label, value)| {
            after_separator
                .strip_prefix(label)
                .map(|rest| (*value, rest))
        }) else {
            return Ok(None);
        };
        self.rest = rest.strip_prefix(['-', '_', '.']).unwrap_or(rest);
        let number = self.number()?.unwrap_or(0);
        Ok(Some((value, number)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Pep440Version {
        Pep440Version::parse(text).unwrap()
    }

    #[test]
    fn the_specification_order() {
        // The ordering example of the specification's "Summary of permitted
        // suffixes and relative ordering", plus epochs and local labels.
        let ordered = [
            "1.dev0",
            "1.0.dev456",
            "1.0a1",
            "1.0a2.dev456",
            "1.0a12.dev456",
            "1.0a12",
            "1.0b1.dev456",
            "1.0b2",
            "1.0b2.post345.dev456",
            "1.0b2.post345",
            "1.0rc1.dev456",
            "1.0rc1",
            "1.0",
            "1.0+abc.5",
            "1.0+abc.7",
            "1.0+5",
            "1.0.post456.dev34",
            "1.0.post456",
            "1.0.15",
            "1.1.dev1",
            "1!0.1",
        ];
        for pair in ordered.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn alternative_spellings_normalize() {
        for (input, normalized) in [
            ("1.0", "1.0"),
            ("v1.0", "1.0"),
            ("1.0ALPHA1", "1.0a1"),
            ("1.0-a.1", "1.0a1"),
            ("1.0_beta", "1.0b0"),
            ("1.0c1", "1.0rc1"),
            ("1.0pre2", "1.0rc2"),
            ("1.0preview2", "1.0rc2"),
            ("1.0-1", "1.0.post1"),
            ("1.0-r4", "1.0.post4"),
            ("1.0rev", "1.0.post0"),
            ("1.0.post", "1.0.post0"),
            ("1.0-dev", "1.0.dev0"),
            ("1.0a.post1", "1.0a0.post1"),
            ("1.0+Ubuntu-1_2", "1.0+ubuntu.1.2"),
            ("01.002", "1.2"),
            ("0!1.0", "1.0"),
            ("2!1.0rc1.post2.dev3+x", "2!1.0rc1.post2.dev3+x"),
        ] {
            let parsed = v(input);
            assert_eq!(parsed.to_string(), normalized, "{input}");
            assert_eq!(parsed.as_str(), input);
            assert_eq!(v(normalized), parsed, "{input}");
            assert_eq!(v(normalized).to_string(), normalized, "idempotent: {input}");
        }
    }

    #[test]
    fn equality_ignores_trailing_zeros_and_spelling() {
        assert_eq!(v("1.0"), v("1.0.0.0"));
        assert_eq!(v("1"), v("1.0"));
        assert_eq!(v("1.0rc1"), v("1.0c1"));
        assert_ne!(v("1.0"), v("1.0+0"));
        assert_ne!(v("1.0"), v("1.0.post0"));
        let hash = |x: &Pep440Version| {
            use std::collections::hash_map::DefaultHasher;
            let mut h = DefaultHasher::new();
            x.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&v("1.0")), hash(&v("1.0.0")));
        assert!(v("1.0a1").is_prerelease());
        assert!(v("1.0.dev1").is_prerelease());
        assert!(!v("1.0.post1").is_prerelease());
    }

    #[test]
    fn rejected() {
        for text in [
            "",
            "v",
            "1.",
            ".1",
            "1..0",
            "1.0-",
            "1.0+",
            "1.0+a..b",
            "1.0+a b",
            " 1.0",
            "1.0 ",
            "1.0a1a1",
            "1.0.post1.post2",
            "1.0dev1.dev2",
            "1!",
            "!1",
            "1.0-x",
            "1.0.x",
            "1.0é",
            "1.0+a/b",
            "1.0+-a",
            "latest",
        ] {
            assert_eq!(
                Pep440Version::parse(text).err(),
                Some(Pep440Error::Invalid),
                "{text:?}"
            );
        }
        assert_eq!(
            Pep440Version::parse("18446744073709551616").err(),
            Some(Pep440Error::NumberTooLarge)
        );
        assert_eq!(
            Pep440Version::parse(&"1.".repeat(600)).err(),
            Some(Pep440Error::TooLong)
        );
    }
}
