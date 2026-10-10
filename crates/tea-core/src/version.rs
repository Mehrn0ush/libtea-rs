//! TEA API version selection.
//!
//! Advertised versions are SemVer 2.0.0. Build metadata does not affect
//! precedence or whether a client supports a version. The selected path uses
//! the advertised spelling exactly, including build metadata.

use std::cmp::Ordering;
use std::fmt;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::error::ModelError;

/// One advertised API version, preserving the wire spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiVersion {
    raw: String,
    parsed: Version,
}

impl ApiVersion {
    /// Parse a SemVer 2.0.0 version string.
    ///
    /// # Errors
    ///
    /// Returns an error when the text is not SemVer 2.0.0.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        let parsed = Version::parse(text).map_err(|err| {
            ModelError::Invalid(format!("API version `{text}` is not SemVer 2.0.0: {err}"))
        })?;
        Ok(Self {
            raw: text.to_owned(),
            parsed,
        })
    }

    /// Advertised spelling, used after `/v` in the path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Parsed version. Comparison ignores build metadata.
    #[must_use]
    pub fn semver(&self) -> &Version {
        &self.parsed
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

/// A version the client implements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedApi {
    versions: Vec<Version>,
}

impl SupportedApi {
    /// Support the pinned TEA 1.0.0 contract.
    #[must_use]
    pub fn tea_1_0_0() -> Self {
        Self {
            versions: vec![Version::new(1, 0, 0)],
        }
    }

    /// Replace the supported set.
    ///
    /// # Errors
    ///
    /// Returns an error when a string is not SemVer 2.0.0.
    pub fn from_versions<I, S>(values: I) -> Result<Self, ModelError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut versions = Vec::new();
        for value in values {
            versions.push(ApiVersion::parse(value.as_ref())?.parsed);
        }
        if versions.is_empty() {
            return Err(ModelError::Invalid(
                "at least one supported API version is required".into(),
            ));
        }
        Ok(Self { versions })
    }

    /// Whether an advertised version is implemented.
    ///
    /// SemVer build metadata is ignored. Prerelease identifiers are not.
    #[must_use]
    pub fn supports(&self, advertised: &Version) -> bool {
        self.versions
            .iter()
            .any(|supported| supported.cmp_precedence(advertised) == Ordering::Equal)
    }
}

impl Default for SupportedApi {
    fn default() -> Self {
        Self::tea_1_0_0()
    }
}

/// Chosen endpoint after version negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedEndpoint {
    /// Absolute API root without a version prefix and without a trailing slash.
    pub root_url: String,
    /// Exact advertised version string to place after `/v`.
    pub version: String,
    /// Priority used for ordering. Omitted priorities are already replaced with `1`.
    pub priority: OrderedPriority,
    /// Index of the endpoint in the document before sorting.
    pub source_index: usize,
}

/// Finite priority in the closed range 0..=1, stored as bits so it can be ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedPriority(u64);

impl OrderedPriority {
    /// Treat an omitted priority as `1`.
    #[must_use]
    pub fn default_priority() -> Self {
        Self(1.0_f64.to_bits())
    }

    /// Validate a priority.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is non-finite or outside 0..=1.
    pub fn parse(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ModelError::Invalid(
                "priority must be a finite number from 0 to 1".into(),
            ));
        }
        Ok(Self(value.to_bits()))
    }

    /// Numeric priority.
    #[must_use]
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// One candidate API base before selection.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionedEndpoint {
    /// API root as advertised.
    pub root_url: String,
    /// Advertised versions, in document order.
    pub versions: Vec<ApiVersion>,
    /// Effective priority.
    pub priority: OrderedPriority,
}

/// Select candidate endpoints in the order a client should try them.
///
/// The order is the highest mutually supported SemVer precedence, then the
/// highest priority, then the original document index. Within one endpoint,
/// tied build-metadata spellings keep the first advertised string.
///
/// # Errors
///
/// Returns an error when no endpoint advertises a supported version.
pub fn select_endpoints(
    endpoints: &[VersionedEndpoint],
    supported: &SupportedApi,
) -> Result<Vec<SelectedEndpoint>, ModelError> {
    let mut selected = Vec::new();
    for (source_index, endpoint) in endpoints.iter().enumerate() {
        let Some(best) = highest_supported(&endpoint.versions, supported) else {
            continue;
        };
        selected.push(SelectedEndpoint {
            root_url: endpoint.root_url.clone(),
            version: best.as_str().to_owned(),
            priority: endpoint.priority,
            source_index,
        });
    }
    if selected.is_empty() {
        return Err(ModelError::Invalid(
            "no endpoint advertises an API version this client supports".into(),
        ));
    }
    selected.sort_by(|left, right| {
        let left_version = Version::parse(&left.version).unwrap_or(Version::new(0, 0, 0));
        let right_version = Version::parse(&right.version).unwrap_or(Version::new(0, 0, 0));
        right_version
            .cmp_precedence(&left_version)
            .then_with(|| right.priority.get().total_cmp(&left.priority.get()))
            .then_with(|| left.source_index.cmp(&right.source_index))
    });
    Ok(selected)
}

fn highest_supported<'a>(
    versions: &'a [ApiVersion],
    supported: &SupportedApi,
) -> Option<&'a ApiVersion> {
    let mut best: Option<&ApiVersion> = None;
    for version in versions {
        if !supported.supports(version.semver()) {
            continue;
        }
        let replace = match best {
            None => true,
            Some(current) => version.semver().cmp_precedence(current.semver()) == Ordering::Greater,
        };
        if replace {
            best = Some(version);
        }
    }
    best
}

/// Build `{root}/v{version}/` without encoding `+` in build metadata.
///
/// # Errors
///
/// Returns an error when `root` is not an absolute base URL or the version is empty.
pub fn versioned_api_prefix(root: &str, version: &str) -> Result<String, ModelError> {
    let root = root.trim_end_matches('/');
    if version.is_empty() || version.contains('/') {
        return Err(ModelError::Invalid(
            "API version path segment is empty".into(),
        ));
    }
    let prefix = format!("{root}/v{version}/");
    let parsed = url::Url::parse(&prefix)
        .map_err(|err| ModelError::Invalid(format!("versioned API URL is invalid: {err}")))?;
    if parsed.path().contains("%2B") || parsed.path().contains("%2b") {
        return Err(ModelError::Invalid(
            "build metadata `+` must remain unencoded in the version path".into(),
        ));
    }
    Ok(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(url: &str, versions: &[&str], priority: f64) -> VersionedEndpoint {
        VersionedEndpoint {
            root_url: url.into(),
            versions: versions
                .iter()
                .map(|v| ApiVersion::parse(v).unwrap())
                .collect(),
            priority: OrderedPriority::parse(priority).unwrap(),
        }
    }

    #[test]
    fn prefers_higher_version_over_priority() {
        let supported = SupportedApi::from_versions(["1.0.0", "1.1.0"]).unwrap();
        let endpoints = vec![
            endpoint("https://low.example", &["1.0.0"], 1.0),
            endpoint("https://high.example", &["1.1.0"], 0.1),
        ];
        let selected = select_endpoints(&endpoints, &supported).unwrap();
        assert_eq!(selected[0].root_url, "https://high.example");
    }

    #[test]
    fn uses_priority_when_versions_match_and_defaults_omitted_priority() {
        let supported = SupportedApi::tea_1_0_0();
        let endpoints = vec![
            endpoint("https://api2.example.com/mytea", &["1.0.0"], 0.5),
            endpoint("https://api.example.com", &["1.0.0"], 1.0),
        ];
        let selected = select_endpoints(&endpoints, &supported).unwrap();
        assert_eq!(selected[0].root_url, "https://api.example.com");
        assert_eq!(selected[0].version, "1.0.0");
    }

    #[test]
    fn keeps_build_metadata_spelling() {
        let supported = SupportedApi::tea_1_0_0();
        let endpoints = vec![endpoint(
            "https://api.example.com",
            &["1.0.0+build.5", "1.0.0+other"],
            1.0,
        )];
        let selected = select_endpoints(&endpoints, &supported).unwrap();
        assert_eq!(selected[0].version, "1.0.0+build.5");
        let prefix = versioned_api_prefix(&selected[0].root_url, &selected[0].version).unwrap();
        assert_eq!(prefix, "https://api.example.com/v1.0.0+build.5/");
    }

    #[test]
    fn prerelease_is_not_the_stable_version() {
        let supported = SupportedApi::tea_1_0_0();
        let endpoints = vec![endpoint("https://api.example.com", &["1.0.0-rc.1"], 1.0)];
        assert!(select_endpoints(&endpoints, &supported).is_err());
    }

    #[test]
    fn absent_priority_beats_lower_explicit_priority() {
        let supported = SupportedApi::tea_1_0_0();
        let mut lower = endpoint("https://b.example", &["1.0.0"], 0.5);
        let mut higher = endpoint("https://a.example", &["1.0.0"], 0.0);
        higher.priority = OrderedPriority::default_priority();
        lower.priority = OrderedPriority::parse(0.5).unwrap();
        let selected = select_endpoints(&[lower, higher], &supported).unwrap();
        assert_eq!(selected[0].root_url, "https://a.example");
    }
}
