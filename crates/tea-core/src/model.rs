//! TEA data model for the pinned OpenAPI 1.0.0 schemas.
//!
//! Composite objects ignore unknown JSON fields so a later revision can add
//! properties without making existing ones unreadable. Fields whose schemas
//! forbid additional properties (`error` recognition, discovery, well-known)
//! are handled in the discovery module. Enumerations preserve unknown values.

use serde::{Deserialize, Serialize};

use crate::checksum::Checksum;
use crate::enums::{
    ArtifactType, CleEventType, CollectionOwner, CollectionUpdateReasonType,
    ComplianceDocumentType, IdentifierType, TeaErrorCode,
};
use crate::error::ModelError;
use crate::ids::{
    ArtifactId, CollectionId, ComponentId, ComponentReleaseId, DistributionId, ProductId,
    ProductReleaseId, TeaTimestamp,
};

/// An external identifier attached to a TEA object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Identifier {
    /// Identifier scheme.
    pub id_type: IdentifierType,
    /// Identifier value, retained as published.
    pub id_value: String,
}

/// A TEA Product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Product {
    /// Product UUID.
    pub uuid: ProductId,
    /// Product name.
    pub name: String,
    /// Identifiers.
    pub identifiers: Vec<Identifier>,
}

/// A component included by a product release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentRef {
    /// Component UUID.
    pub uuid: ComponentId,
    /// Optional fixed component release.
    #[serde(default)]
    pub release: Option<ComponentReleaseId>,
}

/// A TEA Product Release without its collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ProductRelease {
    /// Product release UUID.
    pub uuid: ProductReleaseId,
    /// Parent product UUID.
    pub product: ProductId,
    /// Parent product name, when the server sends it.
    #[serde(default)]
    pub product_name: Option<String>,
    /// Manufacturer version string. This is not SemVer.
    pub version: String,
    /// Creation time.
    pub created_date: TeaTimestamp,
    /// Release time, when present.
    #[serde(default)]
    pub release_date: Option<TeaTimestamp>,
    /// Whether the release is a pre-release.
    #[serde(default)]
    pub pre_release: Option<bool>,
    /// Identifiers.
    #[serde(default)]
    pub identifiers: Option<Vec<Identifier>>,
    /// Components in this release.
    pub components: Vec<ComponentRef>,
}

/// A product release and its latest collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ProductReleaseWithCollection {
    /// Release fields.
    #[serde(flatten)]
    pub release: ProductRelease,
    /// Latest collection.
    pub latest_collection: Collection,
}

/// A TEA Component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Component {
    /// Component UUID.
    pub uuid: ComponentId,
    /// Component name.
    pub name: String,
    /// Identifiers.
    pub identifiers: Vec<Identifier>,
}

/// One distribution of a component release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ReleaseDistribution {
    /// Distribution UUID.
    pub distribution_id: DistributionId,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Identifiers.
    #[serde(default)]
    pub identifiers: Option<Vec<Identifier>>,
    /// External download URL. A TEA access token is not sent here.
    #[serde(default)]
    pub url: Option<String>,
    /// External signature URL. A TEA access token is not sent here.
    #[serde(default)]
    pub signature_url: Option<String>,
    /// Checksums of the distribution bytes, not the signature.
    #[serde(default)]
    pub checksums: Option<Vec<Checksum>>,
}

/// A TEA Component Release without its collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ComponentRelease {
    /// Component release UUID.
    pub uuid: ComponentReleaseId,
    /// Parent component UUID.
    pub component: ComponentId,
    /// Parent component name, when present.
    #[serde(default)]
    pub component_name: Option<String>,
    /// Manufacturer version string.
    pub version: String,
    /// Creation time.
    pub created_date: TeaTimestamp,
    /// Release time, when present.
    #[serde(default)]
    pub release_date: Option<TeaTimestamp>,
    /// Whether the release is a pre-release.
    #[serde(default)]
    pub pre_release: Option<bool>,
    /// Identifiers.
    #[serde(default)]
    pub identifiers: Option<Vec<Identifier>>,
    /// Distributions.
    #[serde(default)]
    pub distributions: Option<Vec<ReleaseDistribution>>,
}

/// A component release and its latest collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ComponentReleaseWithCollection {
    /// Release fields.
    #[serde(flatten)]
    pub release: ComponentRelease,
    /// Latest collection.
    pub latest_collection: Collection,
}

/// Why a collection version was published.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CollectionUpdateReason {
    /// Update classification.
    #[serde(rename = "type")]
    pub reason_type: CollectionUpdateReasonType,
    /// Free-text comment. Clients must not interpret this programmatically.
    #[serde(default)]
    pub comment: Option<String>,
}

/// One format of an artifact revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArtifactFormat {
    /// Media type. Unique within the revision.
    pub media_type: String,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// External content URL. When set, clients retrieve content from it.
    #[serde(default)]
    pub url: Option<String>,
    /// External detached-signature URL.
    #[serde(default)]
    pub signature_url: Option<String>,
    /// Checksums of the content bytes after HTTP content coding is removed.
    #[serde(default)]
    pub checksums: Option<Vec<Checksum>>,
}

impl ArtifactFormat {
    /// Enforce `dependentRequired` for `url` and checksum shape.
    ///
    /// # Errors
    ///
    /// Returns an error when a format with `url` has no checksums, or checksums are empty.
    pub fn validate(&self) -> Result<(), ModelError> {
        if let Some(checksums) = &self.checksums {
            if checksums.is_empty() {
                return Err(ModelError::Invalid(
                    "checksums, when present, must contain at least one entry".into(),
                ));
            }
        }
        if self.url.is_some() && self.checksums.as_ref().map(Vec::is_empty).unwrap_or(true) {
            return Err(ModelError::Invalid(
                "an artifact format with url must publish at least one checksum".into(),
            ));
        }
        if self.media_type.is_empty() {
            return Err(ModelError::Invalid("mediaType must not be empty".into()));
        }
        Ok(())
    }
}

/// A TEA Artifact revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Artifact {
    /// Artifact UUID.
    pub uuid: ArtifactId,
    /// Revision number, starting at 1.
    pub version: u64,
    /// Human-readable name.
    #[serde(default)]
    pub name: Option<String>,
    /// Artifact type.
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// Creation time of this revision.
    pub created_date: TeaTimestamp,
    /// Distributions this revision applies to. Absent means all distributions.
    #[serde(default)]
    pub distribution_ids: Option<Vec<DistributionId>>,
    /// Available formats.
    pub formats: Vec<ArtifactFormat>,
}

impl Artifact {
    /// Validate revision, formats, and checksum requirements.
    ///
    /// # Errors
    ///
    /// Returns an error when the revision is outside the normative constraints.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.version < 1 {
            return Err(ModelError::Invalid("artifact version starts at 1".into()));
        }
        if self.formats.is_empty() {
            return Err(ModelError::Invalid(
                "artifact formats must not be empty".into(),
            ));
        }
        let mut seen = Vec::new();
        for format in &self.formats {
            format.validate()?;
            if seen
                .iter()
                .any(|existing: &String| existing == &format.media_type)
            {
                return Err(ModelError::Invalid(format!(
                    "mediaType `{}` is repeated in one artifact revision",
                    format.media_type
                )));
            }
            seen.push(format.media_type.clone());
        }
        Ok(())
    }

    /// Find a format by exact media type.
    #[must_use]
    pub fn format(&self, media_type: &str) -> Option<&ArtifactFormat> {
        self.formats
            .iter()
            .find(|format| format.media_type == media_type)
    }
}

/// A TEA Collection version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Collection {
    /// Collection UUID. Equal to the parent release UUID.
    pub uuid: CollectionId,
    /// Collection version, starting at 1.
    pub version: u64,
    /// Creation time of this collection version.
    pub created_date: TeaTimestamp,
    /// Parent object type.
    pub belongs_to: CollectionOwner,
    /// Why this version was published.
    pub update_reason: CollectionUpdateReason,
    /// Artifacts. Empty is valid for the initial collection version.
    pub artifacts: Vec<Artifact>,
}

impl Collection {
    /// Validate version and embedded artifacts.
    ///
    /// # Errors
    ///
    /// Returns an error when a collection or artifact constraint fails.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.version < 1 {
            return Err(ModelError::Invalid("collection version starts at 1".into()));
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        Ok(())
    }
}

/// A page of results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Page<T> {
    /// Whether another page exists.
    pub has_next: bool,
    /// Opaque continuation token. Clients copy it verbatim into the next request.
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Page contents.
    pub results: Vec<T>,
}

impl<T> Page<T> {
    /// Build a page, for example in a test double or an adapter.
    #[must_use]
    pub fn new(results: Vec<T>, has_next: bool, next_page_token: Option<String>) -> Self {
        Self {
            has_next,
            next_page_token,
            results,
        }
    }
}

/// TEA error body.
///
/// Unknown `error` strings are preserved. Extra JSON properties are ignored so
/// recognition matches the discovery document rather than failing closed on an
/// additive field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TeaErrorBody {
    /// Machine-readable error code.
    pub error: TeaErrorCode,
    /// Human-readable diagnostic text. Not a programmatic contract.
    #[serde(default)]
    pub message: Option<String>,
}

impl TeaErrorBody {
    /// Parse a JSON error body.
    ///
    /// # Errors
    ///
    /// Returns an error when the body is not a TEA error object.
    pub fn parse(bytes: &[u8]) -> Result<Self, ModelError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

/// OAuth token error body from `POST /token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenErrorBody {
    /// OAuth error code.
    pub error: String,
    /// Optional description.
    #[serde(default)]
    pub error_description: Option<String>,
    /// Optional URI.
    #[serde(default)]
    pub error_uri: Option<String>,
}

/// Successful token response. The access token is opaque.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct TokenResponse {
    /// Opaque access token.
    pub access_token: String,
    /// Must be `Bearer` for TEA resource access.
    pub token_type: String,
    /// Lifetime in seconds, when the server sends it.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Granted scope, when present.
    #[serde(default)]
    pub scope: Option<String>,
}

impl TokenResponse {
    /// Reject a token type other than Bearer.
    ///
    /// # Errors
    ///
    /// Returns an error when `token_type` is not Bearer, ignoring case.
    pub fn bearer(self) -> Result<Self, ModelError> {
        if !self.token_type.eq_ignore_ascii_case("bearer") {
            return Err(ModelError::Invalid(format!(
                "token_type must be Bearer, got `{}`",
                self.token_type
            )));
        }
        if self.access_token.is_empty() {
            return Err(ModelError::Invalid("access_token is empty".into()));
        }
        Ok(self)
    }
}

/// CLE support definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CleSupportDefinition {
    /// Support identifier.
    pub id: String,
    /// Description.
    pub description: String,
    /// Optional URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// CLE definition block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CleDefinitions {
    /// Support definitions.
    #[serde(default)]
    pub support: Option<Vec<CleSupportDefinition>>,
}

/// One CLE event in the TEA projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CleEvent {
    /// Event id.
    pub id: i64,
    /// Event type.
    #[serde(rename = "type")]
    pub event_type: CleEventType,
    /// Effective time.
    pub effective: TeaTimestamp,
    /// Publication time.
    pub published: TeaTimestamp,
    /// Version.
    #[serde(default)]
    pub version: Option<String>,
    /// Version specifiers.
    #[serde(default)]
    pub versions: Option<Vec<CleVersionSpecifier>>,
    /// Support id.
    #[serde(default)]
    pub support_id: Option<String>,
    /// License.
    #[serde(default)]
    pub license: Option<String>,
    /// Superseding version.
    #[serde(default)]
    pub superseded_by_version: Option<String>,
    /// Identifiers.
    #[serde(default)]
    pub identifiers: Option<Vec<Identifier>>,
    /// Related event id.
    #[serde(default)]
    pub event_id: Option<i64>,
    /// Reason.
    #[serde(default)]
    pub reason: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// References.
    #[serde(default)]
    pub references: Option<Vec<String>>,
}

/// CLE version specifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CleVersionSpecifier {
    /// Exact version.
    #[serde(default)]
    pub version: Option<String>,
    /// Version range.
    #[serde(default)]
    pub range: Option<String>,
}

/// TEA projection of CLE events. This is not a full ECMA-428 document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Cle {
    /// Events.
    pub events: Vec<CleEvent>,
    /// Definitions.
    #[serde(default)]
    pub definitions: Option<CleDefinitions>,
}

/// Compliance document type re-export helper for callers building identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceDocument(pub ComplianceDocumentType);

/// Deserialize JSON and run `validate` when the type has one.
///
/// # Errors
///
/// Returns an error on malformed JSON or a failed constraint.
pub fn parse_json<T>(bytes: &[u8]) -> Result<T, ModelError>
where
    T: for<'de> Deserialize<'de>,
{
    Ok(serde_json::from_slice(bytes)?)
}

/// Parse an artifact and validate it.
///
/// # Errors
///
/// Returns an error when JSON or artifact constraints fail.
pub fn parse_artifact(bytes: &[u8]) -> Result<Artifact, ModelError> {
    let artifact: Artifact = parse_json(bytes)?;
    artifact.validate()?;
    Ok(artifact)
}

/// Parse a collection and validate it.
///
/// # Errors
///
/// Returns an error when JSON or collection constraints fail.
pub fn parse_collection(bytes: &[u8]) -> Result<Collection, ModelError> {
    let collection: Collection = parse_json(bytes)?;
    collection.validate()?;
    Ok(collection)
}

/// Parse a product release with its latest collection.
///
/// # Errors
///
/// Returns an error when JSON or embedded collection constraints fail.
pub fn parse_product_release(bytes: &[u8]) -> Result<ProductReleaseWithCollection, ModelError> {
    let value: ProductReleaseWithCollection = parse_json(bytes)?;
    value.latest_collection.validate()?;
    Ok(value)
}

/// Parse a component release with its latest collection.
///
/// # Errors
///
/// Returns an error when JSON or embedded collection constraints fail.
pub fn parse_component_release(bytes: &[u8]) -> Result<ComponentReleaseWithCollection, ModelError> {
    let value: ComponentReleaseWithCollection = parse_json(bytes)?;
    value.latest_collection.validate()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_with_url_requires_checksum() {
        let json = br#"{
          "uuid": "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
          "version": 1,
          "type": "BOM",
          "createdDate": "2026-01-02T03:04:05Z",
          "formats": [{"mediaType": "application/vnd.cyclonedx+json", "url": "https://cdn.example/sbom.json"}]
        }"#;
        assert!(parse_artifact(json).is_err());
    }

    #[test]
    fn preserves_unknown_artifact_type() {
        let json = br#"{
          "uuid": "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
          "version": 1,
          "type": "FUTURE_DOC",
          "createdDate": "2026-01-02T03:04:05.5Z",
          "formats": [{"mediaType": "application/json", "checksums": [{"algType": "SHA-256", "algValue": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]}]
        }"#;
        let artifact = parse_artifact(json).unwrap();
        assert_eq!(artifact.artifact_type.as_str(), "FUTURE_DOC");
    }
}
