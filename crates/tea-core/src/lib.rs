//! Domain types and protocol rules for the Transparency Exchange API consumer.
//!
//! **Independent implementation.** Not an official CycloneDX project release.
//! Not a conformance certificate. [`IntegrityOutcome::Verified`] means declared
//! digests matched **decoded** artifact bytes only — it is **not** publisher
//! authenticity and not cryptographic signature verification.
//!
//! This crate does not open network connections, read the filesystem, or depend
//! on an async runtime. [`libtea_client`](https://docs.rs/libtea-client) performs HTTP.
//!
//! The pinned specification is TEA OpenAPI `1.0.0` from
//! <https://github.com/CycloneDX/transparency-exchange-api> commit
//! `962a0228af1bfcc1232ac3f79239317ff33fdb04`.
//!
//! The crate README follows; its Rust snippets run as doctests.
//!
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

// Direct dependencies so published consumers keep an MSRV-compatible IDNA/ICU stack.
// url alone does not constrain those transitive crates.
use idna as _;
use idna_adapter as _;

pub mod checksum;
pub mod discovery;
pub mod enums;
pub mod error;
pub mod ids;
pub mod model;
pub mod policy;
pub mod tei;
pub mod version;

// Third-party crates whose types appear in this crate's public API. Re-exported
// so callers name the exact version this crate was built against; a major
// version bump of any of them is a breaking change of `libtea-core`.
pub use semver;
pub use url;
pub use uuid;

pub use checksum::{
    verify_checksums, verify_checksums_with_policy, Checksum, ChecksumCheck, ChecksumPolicy,
    ChecksumVerifier, IntegrityOutcome, IntegrityReport,
};
pub use discovery::{is_json_content_type, tea_error_code, DiscoveryInfo, WellKnownDocument};
pub use enums::{ChecksumAlgorithm, SortOrder};
pub use error::{escape_control_chars, ModelError, PolicyError, TeiError};
pub use ids::{
    ArtifactId, CollectionId, ComponentId, ComponentReleaseId, DistributionId, ProductId,
    ProductReleaseId, TeaTimestamp, TeaUuid,
};
pub use model::{
    parse_artifact, parse_collection, parse_component_release, parse_product_release, Artifact,
    ArtifactFormat, Cle, Collection, Component, ComponentRef, ComponentRelease,
    ComponentReleaseWithCollection, Identifier, Page, Product, ProductRelease,
    ProductReleaseWithCollection, TeaErrorBody, TokenResponse,
};
pub use policy::{classify_ip, origin_key, parse_api_base, DestinationPolicy, IpClass};
pub use tei::{encode_query_component, Tei};
pub use version::{
    select_endpoints, versioned_api_prefix, ApiVersion, SelectedEndpoint, SupportedApi,
};

/// Pinned TEA specification identity.
pub mod spec {
    /// Primary specification repository.
    pub const REPOSITORY: &str = "https://github.com/CycloneDX/transparency-exchange-api";
    /// Branch that was audited.
    pub const BRANCH: &str = "main";
    /// Commit SHA of the audited revision.
    pub const COMMIT: &str = "962a0228af1bfcc1232ac3f79239317ff33fdb04";
    /// Committer date of that commit.
    pub const COMMIT_DATE: &str = "2026-10-08T06:28:26Z";
    /// Retrieval date used for this SDK.
    pub const RETRIEVAL_DATE: &str = "2026-10-10";
    /// `info.version` of the OpenAPI document.
    pub const SPEC_VERSION: &str = "1.0.0";
    /// OpenAPI version of that document.
    pub const OPENAPI_VERSION: &str = "3.1.1";
}

/// Query parameters shared by list operations.
///
/// [`Default`] leaves every field unset. The request then carries no list
/// query parameters, and the server applies its own page size and sort. `S`
/// does not need to implement [`Default`], so this works for [`enums::ComponentSort`]
/// and the other built-in sort enums.
///
/// # Continuing a page
///
/// Keep the same [`ListParams`] value and replace only [`page_token`](Self::page_token)
/// with [`Page::next_page_token`](crate::Page::next_page_token). That preserves
/// `pageSize`, sort fields, and identifier filters. Building a new value that
/// sets only the token drops those fields and can change the result set.
///
/// Callers must detect pagination anomalies themselves (empty intermediate pages,
/// repeated tokens, missing tokens when `has_next` is true, or server errors).
/// This crate does not auto-paginate and does not break loops on a repeated token.
///
/// ```ignore
/// let mut query = ListParams::default()
///     .with_page_size(1)
///     .with_id_type(IdentifierType::Purl);
/// let first = service.query_products(&query).await?;
/// query.page_token = first.next_page_token.clone();
/// let second = service.query_products(&query).await?;
/// ```
///
/// The struct is `#[non_exhaustive]`: build it with [`ListParams::default`]
/// and the `with_*` setters, or assign the public fields of a default value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListParams<S> {
    /// `pageSize`, from 1 to 100.
    pub page_size: Option<u32>,
    /// Opaque `pageToken` copied from the previous page.
    pub page_token: Option<String>,
    /// Resource-specific `sortField`.
    pub sort_field: Option<S>,
    /// `sortOrder`.
    pub sort_order: Option<enums::SortOrder>,
    /// `idType` filter.
    ///
    /// This may be sent without [`id_value`](Self::id_value). TEA 1.0 uses that
    /// form for compliance-document discovery (`idType=COMPLIANCE_DOCUMENT`).
    pub id_type: Option<enums::IdentifierType>,
    /// `idValue` filter.
    ///
    /// Sent only together with [`id_type`](Self::id_type). This SDK rejects a
    /// value with no type before the request. The pinned specification does
    /// not settle that value-only case, so the rejection is not a claim that
    /// TEA 1.0 forbids it.
    pub id_value: Option<String>,
}

impl<S> Default for ListParams<S> {
    fn default() -> Self {
        Self {
            page_size: None,
            page_token: None,
            sort_field: None,
            sort_order: None,
            id_type: None,
            id_value: None,
        }
    }
}

impl<S> ListParams<S> {
    /// Set `pageSize`.
    #[must_use]
    pub fn with_page_size(mut self, size: u32) -> Self {
        self.page_size = Some(size);
        self
    }

    /// Set the opaque `pageToken` from a previous page.
    #[must_use]
    pub fn with_page_token(mut self, token: impl Into<String>) -> Self {
        self.page_token = Some(token.into());
        self
    }

    /// Set `sortField`.
    #[must_use]
    pub fn with_sort_field(mut self, field: S) -> Self {
        self.sort_field = Some(field);
        self
    }

    /// Set `sortOrder`.
    #[must_use]
    pub fn with_sort_order(mut self, order: enums::SortOrder) -> Self {
        self.sort_order = Some(order);
        self
    }

    /// Set the `idType` filter.
    #[must_use]
    pub fn with_id_type(mut self, id_type: enums::IdentifierType) -> Self {
        self.id_type = Some(id_type);
        self
    }

    /// Set the `idValue` filter. Requires [`Self::with_id_type`] as well.
    #[must_use]
    pub fn with_id_value(mut self, id_value: impl Into<String>) -> Self {
        self.id_value = Some(id_value.into());
        self
    }

    /// Validate pagination bounds and identifier filters.
    ///
    /// `idType` alone is accepted. Both fields together are accepted. Neither
    /// field is accepted. `idValue` without `idType` is rejected.
    ///
    /// # Errors
    ///
    /// Returns an error when `pageSize` is outside 1..=100 or `idValue` is set
    /// without `idType`.
    pub fn validate(&self) -> Result<(), ModelError> {
        if let Some(size) = self.page_size {
            if !(1..=100).contains(&size) {
                return Err(ModelError::Invalid("pageSize must be from 1 to 100".into()));
            }
        }
        if self.id_value.is_some() && self.id_type.is_none() {
            return Err(ModelError::Invalid("idValue requires idType".into()));
        }
        Ok(())
    }
}

#[cfg(doctest)]
mod non_exhaustive_guards;

#[cfg(test)]
mod tests {
    use super::ListParams;
    use crate::enums::{ComponentSort, IdentifierType, SortOrder};

    struct SortWithoutDefault;

    fn params(id_type: Option<IdentifierType>, id_value: Option<&str>) -> ListParams<()> {
        ListParams {
            page_size: None,
            page_token: None,
            sort_field: None,
            sort_order: None,
            id_type,
            id_value: id_value.map(str::to_owned),
        }
    }

    #[test]
    fn neither_identifier_field_is_accepted() {
        assert!(params(None, None).validate().is_ok());
    }

    #[test]
    fn both_identifier_fields_are_accepted() {
        assert!(
            params(Some(IdentifierType::Purl), Some("pkg:generic/widget@1"))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn id_type_without_id_value_is_accepted() {
        assert!(params(Some(IdentifierType::ComplianceDocument), None)
            .validate()
            .is_ok());
    }

    #[test]
    fn id_value_without_id_type_requires_id_type() {
        let error = params(None, Some("pkg:generic/widget@1"))
            .validate()
            .unwrap_err();
        assert_eq!(error.to_string(), "idValue requires idType");
    }

    #[test]
    fn default_leaves_every_field_unset_without_a_default_sort() {
        let query = ListParams::<SortWithoutDefault>::default();
        assert!(query.page_size.is_none());
        assert!(query.page_token.is_none());
        assert!(query.sort_field.is_none());
        assert!(query.sort_order.is_none());
        assert!(query.id_type.is_none());
        assert!(query.id_value.is_none());
        assert!(query.validate().is_ok());

        let components = ListParams::<ComponentSort>::default();
        assert!(components.sort_field.is_none());
        assert!(components.validate().is_ok());

        let value_only = ListParams::<ComponentSort> {
            id_value: Some("pkg:generic/widget@1".to_owned()),
            ..ListParams::default()
        };
        assert_eq!(
            value_only.validate().unwrap_err().to_string(),
            "idValue requires idType"
        );
    }

    #[test]
    fn page_size_bounds_are_unchanged_for_a_type_only_filter() {
        let mut query = params(Some(IdentifierType::ComplianceDocument), None);
        query.page_size = Some(0);
        query.sort_order = Some(SortOrder::Desc);
        assert!(query.validate().is_err());
        query.page_size = Some(101);
        assert!(query.validate().is_err());
        query.page_size = Some(1);
        assert!(query.validate().is_ok());
        query.page_size = Some(100);
        assert!(query.validate().is_ok());
    }
}
