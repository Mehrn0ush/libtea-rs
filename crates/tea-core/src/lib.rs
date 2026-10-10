//! Domain types and protocol rules for the Transparency Exchange API consumer.
//!
//! This crate does not open network connections, read the filesystem, or depend
//! on an async runtime. [`tea_client`](https://docs.rs/tea-client) performs HTTP.
//!
//! The pinned specification is TEA OpenAPI `1.0.0` from
//! <https://github.com/CycloneDX/transparency-exchange-api> commit
//! `962a0228af1bfcc1232ac3f79239317ff33fdb04`.

#![forbid(unsafe_code)]

pub mod checksum;
pub mod discovery;
pub mod enums;
pub mod error;
pub mod ids;
pub mod model;
pub mod policy;
pub mod tei;
pub mod version;

pub use checksum::{
    verify_checksums, Checksum, ChecksumVerifier, IntegrityOutcome, IntegrityReport,
};
pub use discovery::{is_json_content_type, tea_error_code, DiscoveryInfo, WellKnownDocument};
pub use enums::{ChecksumAlgorithm, SortOrder};
pub use error::{ModelError, PolicyError, TeiError};
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListParams<S> {
    /// `pageSize`, from 1 to 100.
    pub page_size: Option<u32>,
    /// Opaque `pageToken` copied from the previous page.
    pub page_token: Option<String>,
    /// Resource-specific `sortField`.
    pub sort_field: Option<S>,
    /// `sortOrder`.
    pub sort_order: Option<enums::SortOrder>,
    /// `idType` filter. Must be paired with `id_value`.
    pub id_type: Option<enums::IdentifierType>,
    /// `idValue` filter. Must be paired with `id_type`.
    pub id_value: Option<String>,
}

impl<S> ListParams<S> {
    /// Validate pagination bounds and identifier filter pairing.
    ///
    /// # Errors
    ///
    /// Returns an error when `pageSize` is outside 1..=100 or only one of
    /// `idType` and `idValue` is set.
    pub fn validate(&self) -> Result<(), ModelError> {
        if let Some(size) = self.page_size {
            if !(1..=100).contains(&size) {
                return Err(ModelError::Invalid("pageSize must be from 1 to 100".into()));
            }
        }
        if self.id_type.is_some() != self.id_value.is_some() {
            return Err(ModelError::Invalid(
                "idType and idValue must be provided together".into(),
            ));
        }
        Ok(())
    }
}
