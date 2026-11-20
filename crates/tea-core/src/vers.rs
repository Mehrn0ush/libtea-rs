//! `vers` version ranges (the version-range notation CLE and PURL use),
//! parsed in full and evaluated for SemVer and PyPI versions.
//!
//! **Experimental:** may change in a 0.x minor release.
//!
//! [`VersRange::parse`] checks the syntax of any `vers:<type>/<constraints>`
//! string. [`VersRange::contains_semver`] then evaluates it when its type
//! compares versions as SemVer 2.0 ([`VersRange::is_semver_type`]): `semver`,
//! and `npm`, whose registered `vers` type uses node-semver versions.
//! [`VersRange::contains_pypi`] evaluates the `pypi` type with PEP 440
//! ordering ([`crate::pep440`]). Every other type is reported as unsupported,
//! never guessed.
//!
//! The rules are from the `vers` specification, release `v1.2.1` of
//! <https://github.com/package-url/vers-spec> (`docs/specification/`):
//! constraints are sorted by version, each version occurs once, `*` stands
//! alone, and outside `!=` the comparators alternate between lower and upper
//! bounds. A range that breaks these rules is an error, not corrected.

use std::cmp::Ordering;

use thiserror::Error;

use crate::pep440::Pep440Version;

/// Inputs longer than this are rejected with [`VersError::TooLong`].
pub const MAX_VERS_BYTES: usize = 4096;

/// Why a string is not a valid `vers` range, or could not be evaluated.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum VersError {
    /// Longer than [`MAX_VERS_BYTES`].
    #[error("vers range is longer than {MAX_VERS_BYTES} bytes")]
    TooLong,
    /// The scheme is missing or is not lowercase `vers`.
    #[error("vers range must start with `vers:`")]
    Scheme,
    /// The type is missing, not lowercase, does not start with a letter, or
    /// has characters other than ASCII letters, digits, `.` and `-`.
    #[error("vers type is missing or invalid")]
    Type,
    /// Nothing after `vers:<type>/`.
    #[error("vers constraints are empty")]
    Empty,
    /// Literal ASCII whitespace anywhere in the string, or a decoded version
    /// with whitespace other than a space.
    #[error("vers range contains whitespace")]
    Whitespace,
    /// A leading, trailing or repeated `|`.
    #[error("vers constraints have an empty segment")]
    EmptyConstraint,
    /// A constraint starts with `=`; equality is a bare version.
    #[error("vers constraint starts with `=`")]
    LeadingEquals,
    /// A comparator with no version after it.
    #[error("vers constraint has no version")]
    EmptyVersion,
    /// `*` together with other constraints, or `*` followed by a version.
    #[error("vers `*` must stand alone")]
    Asterisk,
    /// A `%` not followed by two hex digits, decoded bytes that are not UTF-8,
    /// or an unencoded character the specification reserves.
    #[error("vers version has an invalid percent-encoding")]
    Encoding,
    /// The type is not one the evaluating method handles (SemVer types for
    /// [`VersRange::contains_semver`], `pypi` for [`VersRange::contains_pypi`]).
    #[error("vers type is not evaluated by this method")]
    UnsupportedType,
    /// A constraint version is not a SemVer 2.0 version.
    #[error("vers version is not a SemVer version")]
    NotSemver,
    /// Constraints are not sorted by version, or a version occurs twice.
    #[error("vers constraints are not sorted by version or repeat one")]
    Order,
    /// Comparators do not alternate between lower and upper bounds.
    #[error("vers constraints have an invalid comparator sequence")]
    Sequence,
    /// A constraint version of a `pypi` range is not a PEP 440 version.
    #[error("vers version is not a PEP 440 version")]
    NotPep440,
}

/// The comparator of one constraint.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Comparator {
    /// A bare version: equal to it.
    Equal,
    /// `!=`: excluded.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

impl Comparator {
    /// The comparator as written in a range (empty for [`Self::Equal`]).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }

    fn is_upper(self) -> bool {
        matches!(self, Self::Less | Self::LessOrEqual)
    }

    fn is_lower(self) -> bool {
        matches!(self, Self::Greater | Self::GreaterOrEqual)
    }
}

/// One constraint: a comparator and a percent-decoded version.
///
/// **Experimental:** may change in a 0.x minor release.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Constraint {
    comparator: Comparator,
    version: String,
}

impl Constraint {
    /// The comparator.
    #[must_use]
    pub fn comparator(&self) -> Comparator {
        self.comparator
    }

    /// The version, percent-decoded once.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// A parsed `vers` range.
///
/// **Experimental:** may change in a 0.x minor release.
///
/// ```
/// use libtea_core::vers::VersRange;
///
/// let range = VersRange::parse("vers:npm/>=1.0.0|!=1.2.0|<2.0.0").unwrap();
/// let v = |s: &str| semver::Version::parse(s).unwrap();
/// assert_eq!(range.contains_semver(&v("1.5.0")), Ok(true));
/// assert_eq!(range.contains_semver(&v("1.2.0")), Ok(false));
/// assert_eq!(range.contains_semver(&v("2.0.0")), Ok(false));
/// assert_eq!(range.contains_semver(&v("2.0.0-rc.1")), Ok(true));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersRange {
    version_type: String,
    any: bool,
    constraints: Vec<Constraint>,
}

impl VersRange {
    /// Parse `vers:<type>/<constraints>` as the specification's
    /// "How to parse" describes. Ordering and comparator rules depend on how
    /// the type compares versions, so they are checked by
    /// [`Self::contains_semver`], not here.
    ///
    /// # Errors
    ///
    /// Returns [`VersError`] for a missing or wrong scheme or type, empty
    /// constraints or constraint segments, whitespace, a leading `=`, an empty
    /// version, a misplaced `*`, invalid percent-encoding, or a string longer
    /// than [`MAX_VERS_BYTES`].
    pub fn parse(text: &str) -> Result<Self, VersError> {
        if text.len() > MAX_VERS_BYTES {
            return Err(VersError::TooLong);
        }
        if text.bytes().any(|b| b.is_ascii_whitespace()) {
            return Err(VersError::Whitespace);
        }
        let (scheme, specifier) = text.split_once(':').ok_or(VersError::Scheme)?;
        if scheme != "vers" {
            return Err(VersError::Scheme);
        }
        let (version_type, constraints) = specifier.split_once('/').ok_or(VersError::Type)?;
        if !valid_type(version_type) {
            return Err(VersError::Type);
        }
        if constraints.is_empty() {
            return Err(VersError::Empty);
        }
        if constraints == "*" {
            return Ok(Self {
                version_type: version_type.to_owned(),
                any: true,
                constraints: Vec::new(),
            });
        }
        let mut parsed = Vec::new();
        for segment in constraints.split('|') {
            if segment.is_empty() {
                return Err(VersError::EmptyConstraint);
            }
            if segment.contains('*') {
                return Err(VersError::Asterisk);
            }
            let (comparator, rest) = split_comparator(segment)?;
            if rest.is_empty() {
                return Err(VersError::EmptyVersion);
            }
            if rest.contains(['<', '>', '=', '!']) {
                return Err(VersError::Encoding);
            }
            let version = percent_decode(rest)?;
            // Only an encoded space is allowed; `%09` is still a tab.
            if version
                .bytes()
                .any(|b| b.is_ascii_whitespace() && b != b' ')
            {
                return Err(VersError::Whitespace);
            }
            parsed.push(Constraint {
                comparator,
                version,
            });
        }
        Ok(Self {
            version_type: version_type.to_owned(),
            any: false,
            constraints: parsed,
        })
    }

    /// The `vers` type, such as `npm` or `semver`.
    #[must_use]
    pub fn version_type(&self) -> &str {
        &self.version_type
    }

    /// Whether the range is `*` (every version).
    #[must_use]
    pub fn is_any(&self) -> bool {
        self.any
    }

    /// The constraints in the order written. Empty for `*`.
    #[must_use]
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Whether this type's versions are compared as SemVer 2.0 here:
    /// `semver`, and `npm` (node-semver versions).
    #[must_use]
    pub fn is_semver_type(&self) -> bool {
        matches!(self.version_type.as_str(), "semver" | "npm")
    }

    /// Whether `version` is in the range, comparing as SemVer 2.0 (build
    /// metadata ignored, pre-releases ordered before their release).
    ///
    /// The range is validated first: constraint versions must be SemVer,
    /// sorted, unique, and in a valid comparator sequence.
    ///
    /// # Errors
    ///
    /// [`VersError::UnsupportedType`] when [`Self::is_semver_type`] is false,
    /// [`VersError::NotSemver`] for a constraint version that does not parse,
    /// and [`VersError::Order`] or [`VersError::Sequence`] for a range the
    /// specification calls invalid.
    pub fn contains_semver(&self, version: &semver::Version) -> Result<bool, VersError> {
        if !self.is_semver_type() {
            return Err(VersError::UnsupportedType);
        }
        self.contains_with(
            version,
            |text| semver::Version::parse(text).map_err(|_| VersError::NotSemver),
            precedence,
        )
    }

    /// Whether this range's versions are compared as PEP 440 versions here:
    /// the `pypi` type.
    #[must_use]
    pub fn is_pypi_type(&self) -> bool {
        self.version_type == "pypi"
    }

    /// Whether `version` is in the range, comparing with PEP 440 ordering
    /// ([`crate::pep440`]): `1.0 == 1.0.0`, and
    /// `1.0.dev1 < 1.0a1 < 1.0 < 1.0.post1 < 1.0.1`.
    ///
    /// Only the ordering is used, as `vers` asks: PEP 440's own specifier
    /// rules (such as `<1.0` excluding `1.0a1`) do not apply, so
    /// `vers:pypi/<1.0` contains `1.0a1`. The range is validated first, as
    /// for [`Self::contains_semver`].
    ///
    /// ```
    /// use libtea_core::pep440::Pep440Version;
    /// use libtea_core::vers::VersRange;
    ///
    /// let range = VersRange::parse("vers:pypi/>=1.0|<2.0").unwrap();
    /// let v = |s: &str| Pep440Version::parse(s).unwrap();
    /// assert_eq!(range.contains_pypi(&v("1.5.post1")), Ok(true));
    /// assert_eq!(range.contains_pypi(&v("1.0.0")), Ok(true));
    /// assert_eq!(range.contains_pypi(&v("2.0rc1")), Ok(true));
    /// assert_eq!(range.contains_pypi(&v("2.0")), Ok(false));
    /// ```
    ///
    /// # Errors
    ///
    /// [`VersError::UnsupportedType`] when [`Self::is_pypi_type`] is false,
    /// [`VersError::NotPep440`] for a constraint version that does not parse,
    /// and [`VersError::Order`] or [`VersError::Sequence`] for a range the
    /// specification calls invalid (including `1.0|1.0.0`, which repeats a
    /// version).
    pub fn contains_pypi(&self, version: &Pep440Version) -> Result<bool, VersError> {
        if !self.is_pypi_type() {
            return Err(VersError::UnsupportedType);
        }
        self.contains_with(
            version,
            |text| Pep440Version::parse(text).map_err(|_| VersError::NotPep440),
            Ord::cmp,
        )
    }

    /// Validate the constraints and evaluate them for `version`, with
    /// `parse` for constraint versions and `order` for comparing them.
    fn contains_with<T>(
        &self,
        version: &T,
        parse: impl Fn(&str) -> Result<T, VersError>,
        order: impl Fn(&T, &T) -> Ordering,
    ) -> Result<bool, VersError> {
        if self.any {
            return Ok(true);
        }
        let mut bounds = Vec::with_capacity(self.constraints.len());
        for constraint in &self.constraints {
            bounds.push((constraint.comparator, parse(&constraint.version)?));
        }
        validate(&bounds, &order)?;

        // Equality first: an inclusive constraint on the version includes it,
        // a `!=` on it excludes it. Versions are unique, so these never clash.
        for (comparator, bound) in &bounds {
            if order(version, bound) == Ordering::Equal {
                // `!=` excludes it; `<` and `>` do not include their own version.
                return Ok(!matches!(
                    comparator,
                    Comparator::NotEqual | Comparator::Less | Comparator::Greater
                ));
            }
        }
        // Then the intervals the remaining bounds describe, in version order.
        let ranges: Vec<_> = bounds
            .iter()
            .filter(|(comparator, _)| comparator.is_lower() || comparator.is_upper())
            .collect();
        for (index, (comparator, bound)) in ranges.iter().enumerate() {
            let below = order(version, bound) == Ordering::Less;
            if comparator.is_upper() {
                // An upper bound opens the range when it is the first bound,
                // or closes the interval the previous lower bound opened.
                let opened = match index.checked_sub(1).map(|i| ranges[i]) {
                    None => true,
                    Some((previous, lower)) => {
                        previous.is_lower() && order(version, lower) == Ordering::Greater
                    }
                };
                if opened && below {
                    return Ok(true);
                }
            } else if index + 1 == ranges.len() && order(version, bound) == Ordering::Greater {
                // A lower bound with nothing after it is open to the top.
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// SemVer 2.0 precedence: build metadata does not count.
fn precedence(a: &semver::Version, b: &semver::Version) -> Ordering {
    (a.major, a.minor, a.patch, &a.pre).cmp(&(b.major, b.minor, b.patch, &b.pre))
}

/// Specification section 5.4: sorted and unique, then the comparator rules
/// for contiguous constraints.
fn validate<T>(
    bounds: &[(Comparator, T)],
    order: impl Fn(&T, &T) -> Ordering,
) -> Result<(), VersError> {
    if bounds
        .windows(2)
        .any(|pair| order(&pair[0].1, &pair[1].1) != Ordering::Less)
    {
        return Err(VersError::Order);
    }
    let without_unequal: Vec<_> = bounds
        .iter()
        .map(|(comparator, _)| *comparator)
        .filter(|comparator| *comparator != Comparator::NotEqual)
        .collect();
    // A bare version is followed only by a bare version or a lower bound, and
    // a lower bound is never followed by a bare version (it would be inside
    // the interval already, which "How to parse" calls redundant).
    for pair in without_unequal.windows(2) {
        if (pair[0] == Comparator::Equal && pair[1].is_upper())
            || (pair[0].is_lower() && pair[1] == Comparator::Equal)
        {
            return Err(VersError::Sequence);
        }
    }
    let bounds_only: Vec<_> = without_unequal
        .iter()
        .filter(|comparator| **comparator != Comparator::Equal)
        .collect();
    for pair in bounds_only.windows(2) {
        if pair[0].is_upper() == pair[1].is_upper() {
            return Err(VersError::Sequence);
        }
    }
    Ok(())
}

fn split_comparator(segment: &str) -> Result<(Comparator, &str), VersError> {
    for (prefix, comparator) in [
        (">=", Comparator::GreaterOrEqual),
        ("<=", Comparator::LessOrEqual),
        ("!=", Comparator::NotEqual),
        ("<", Comparator::Less),
        (">", Comparator::Greater),
    ] {
        if let Some(rest) = segment.strip_prefix(prefix) {
            return Ok((comparator, rest));
        }
    }
    if segment.starts_with('=') {
        return Err(VersError::LeadingEquals);
    }
    Ok((Comparator::Equal, segment))
}

/// Section 5.3.2: lowercase ASCII letters, digits, `.` and `-`, starting with a letter.
fn valid_type(version_type: &str) -> bool {
    let mut chars = version_type.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-'))
}

fn percent_decode(text: &str) -> Result<String, VersError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let digit = |offset: usize| {
                bytes
                    .get(index + offset)
                    .and_then(|b| char::from(*b).to_digit(16))
            };
            let (Some(high), Some(low)) = (digit(1), digit(2)) else {
                return Err(VersError::Encoding);
            };
            out.push((high * 16 + low) as u8);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| VersError::Encoding)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> semver::Version {
        semver::Version::parse(text).unwrap()
    }

    fn contains(range: &str, version: &str) -> bool {
        VersRange::parse(range)
            .unwrap()
            .contains_semver(&v(version))
            .unwrap()
    }

    #[test]
    fn single_bounds_and_equality() {
        assert!(contains("vers:semver/>=1.0.0", "1.0.0"));
        assert!(contains("vers:semver/>=1.0.0", "9.0.0"));
        assert!(!contains("vers:semver/>=1.0.0", "0.9.9"));
        assert!(!contains("vers:semver/>1.0.0", "1.0.0"));
        assert!(contains("vers:semver/<2.0.0", "0.0.1"));
        assert!(!contains("vers:semver/<2.0.0", "2.0.0"));
        assert!(contains("vers:semver/<=2.0.0", "2.0.0"));
        assert!(contains("vers:semver/1.2.3", "1.2.3"));
        assert!(!contains("vers:semver/1.2.3", "1.2.4"));
        assert!(contains("vers:semver/*", "0.0.0"));
    }

    #[test]
    fn several_intervals_exclusions_and_points() {
        let range = "vers:npm/<1.4.1|>=2.0.0|<2.17.2";
        assert!(contains(range, "1.0.0"));
        assert!(!contains(range, "1.4.1"));
        assert!(!contains(range, "1.9.0"));
        assert!(contains(range, "2.0.0"));
        assert!(contains(range, "2.17.1"));
        assert!(!contains(range, "2.17.2"));
        let gaps = "vers:npm/>=2.2.6|<2.47.0|>2.51.0|<=2.67.0";
        assert!(!contains(gaps, "2.51.0"));
        assert!(contains(gaps, "2.60.0"));
        assert!(contains(gaps, "2.67.0"));
        assert!(!contains(gaps, "2.68.0"));
        let points = "vers:semver/1.0.0|1.1.0|>=2.0.0";
        assert!(contains(points, "1.1.0"));
        assert!(!contains(points, "1.0.5"));
        assert!(contains(points, "3.0.0"));
        let excluded = "vers:semver/>=1.0.0|!=1.5.0|<2.0.0";
        assert!(!contains(excluded, "1.5.0"));
        assert!(contains(excluded, "1.5.1"));
    }

    #[test]
    fn prerelease_and_build_metadata() {
        assert!(contains("vers:semver/<1.0.0", "1.0.0-rc.1"));
        assert!(contains("vers:semver/>=1.0.0-rc.1|<1.0.0", "1.0.0-rc.2"));
        // Build metadata is not part of precedence.
        assert!(contains("vers:semver/1.0.0", "1.0.0+build.7"));
        assert!(contains("vers:semver/1.0.0+a", "1.0.0"));
    }

    #[test]
    fn invalid_ranges_are_errors_not_corrected() {
        let check = |range: &str| {
            VersRange::parse(range)
                .and_then(|r| r.contains_semver(&v("1.0.0")))
                .unwrap_err()
        };
        assert_eq!(check("vers:semver/<2.0.0|>=1.0.0"), VersError::Order);
        assert_eq!(check("vers:semver/>=1.0.0|<1.0.0"), VersError::Order);
        assert_eq!(check("vers:semver/>=1.0.0|>=2.0.0"), VersError::Sequence);
        assert_eq!(check("vers:semver/<1.0.0|<2.0.0"), VersError::Sequence);
        assert_eq!(check("vers:semver/1.0.0|<2.0.0"), VersError::Sequence);
        assert_eq!(check("vers:semver/>=1.0.0|1.5.0"), VersError::Sequence);
        assert_eq!(
            check("vers:semver/>=1.0.0|1.5.0|<2.0.0"),
            VersError::Sequence
        );
        assert_eq!(check("vers:semver/1.0"), VersError::NotSemver);
        assert_eq!(check("vers:pypi/>=1.0"), VersError::UnsupportedType);
    }

    fn contains_pypi(range: &str, version: &str) -> Result<bool, VersError> {
        VersRange::parse(range)
            .unwrap()
            .contains_pypi(&Pep440Version::parse(version).unwrap())
    }

    #[test]
    fn pypi_ranges_use_pep440_ordering() {
        let range = "vers:pypi/>=1.0|!=1.5|<2.0";
        assert_eq!(contains_pypi(range, "1.0.0"), Ok(true));
        assert_eq!(contains_pypi(range, "1.0.dev1"), Ok(false));
        assert_eq!(contains_pypi(range, "1.5.0"), Ok(false));
        assert_eq!(contains_pypi(range, "1.5.post1"), Ok(true));
        assert_eq!(contains_pypi(range, "2.0rc1"), Ok(true));
        assert_eq!(contains_pypi(range, "2.0"), Ok(false));
        assert_eq!(contains_pypi(range, "1!0.1"), Ok(false));
        assert_eq!(contains_pypi("vers:pypi/1.0", "1.0.0+local"), Ok(false));
        assert_eq!(contains_pypi("vers:pypi/*", "0"), Ok(true));
        // Spelled differently, the same version twice.
        assert_eq!(
            contains_pypi("vers:pypi/1.0|1.0.0", "1"),
            Err(VersError::Order)
        );
        assert_eq!(
            contains_pypi("vers:pypi/2.0|<1.0", "1"),
            Err(VersError::Order)
        );
        assert_eq!(
            contains_pypi("vers:pypi/>=1.0|latest", "1"),
            Err(VersError::NotPep440)
        );
        assert_eq!(
            VersRange::parse("vers:npm/1.0.0")
                .unwrap()
                .contains_pypi(&Pep440Version::parse("1.0.0").unwrap()),
            Err(VersError::UnsupportedType)
        );
        assert_eq!(
            VersRange::parse("vers:pypi/1.0.0")
                .unwrap()
                .contains_semver(&v("1.0.0")),
            Err(VersError::UnsupportedType)
        );
    }

    #[test]
    fn syntax_errors() {
        let cases: &[(&str, VersError)] = &[
            ("npm/>=1.0.0", VersError::Scheme),
            ("VERS:npm/1.0.0", VersError::Scheme),
            ("vers:NPM/1.0.0", VersError::Type),
            ("vers:1npm/1.0.0", VersError::Type),
            ("vers:npm", VersError::Type),
            ("vers:npm/", VersError::Empty),
            ("vers:npm/>=1.0.0| <2.0.0", VersError::Whitespace),
            ("vers:npm/|1.0.0", VersError::EmptyConstraint),
            ("vers:npm/1.0.0|", VersError::EmptyConstraint),
            ("vers:npm/1.0.0||2.0.0", VersError::EmptyConstraint),
            ("vers:npm/=1.0.0", VersError::LeadingEquals),
            ("vers:npm/>=", VersError::EmptyVersion),
            ("vers:npm/*|1.0.0", VersError::Asterisk),
            ("vers:npm/>=*", VersError::Asterisk),
            ("vers:npm/1.0%zz", VersError::Encoding),
            ("vers:npm/>==1.0.0", VersError::Encoding),
        ];
        for (range, error) in cases {
            assert_eq!(
                VersRange::parse(range).as_ref().err(),
                Some(error),
                "{range}"
            );
        }
    }

    #[test]
    fn versions_are_decoded_once() {
        let range = VersRange::parse("vers:generic/1.0%2520a").unwrap();
        assert_eq!(range.constraints()[0].version(), "1.0%20a");
        assert_eq!(range.constraints()[0].comparator().as_str(), "");
    }
}
