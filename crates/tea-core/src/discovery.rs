//! `.well-known/tea` documents and `/discovery` responses.
//!
//! Parsing enforces the pinned schema constraints that decide failover:
//! `schemaVersion` 1, non-empty endpoint lists, absolute base URLs, and
//! SemVer versions. Priority omitted on the wire becomes `1`.

use serde::Deserialize;
use url::Url;

use crate::error::{ModelError, PolicyError};
use crate::ids::ProductReleaseId;
use crate::policy::parse_api_base;
use crate::version::{
    select_endpoints, ApiVersion, OrderedPriority, SelectedEndpoint, SupportedApi,
    VersionedEndpoint,
};

/// A validated `.well-known/tea` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WellKnownDocument {
    endpoints: Vec<WellKnownEndpoint>,
}

/// One well-known endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WellKnownEndpoint {
    /// Advertised API root.
    pub url: String,
    /// Advertised versions in document order.
    pub versions: Vec<ApiVersion>,
    /// Effective priority.
    pub priority: OrderedPriority,
}

/// One resolved product release from `/discovery`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiscoveryInfo {
    /// Product release UUID.
    pub product_release: ProductReleaseId,
    /// Servers that can serve that release.
    pub servers: Vec<DiscoveryServer>,
}

/// One `servers[]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiscoveryServer {
    /// API root.
    pub root_url: String,
    /// Advertised versions in document order.
    pub versions: Vec<ApiVersion>,
    /// Effective priority.
    pub priority: OrderedPriority,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WellKnownWire {
    schema_version: i64,
    endpoints: Vec<EndpointWire>,
}

#[derive(Debug, Deserialize)]
struct EndpointWire {
    url: String,
    versions: Vec<String>,
    #[serde(default)]
    priority: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryWire {
    product_release_uuid: String,
    servers: Vec<ServerWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerWire {
    root_url: String,
    versions: Vec<String>,
    #[serde(default)]
    priority: Option<f64>,
}

impl WellKnownDocument {
    /// Parse and validate a well-known document.
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON is not a schema version 1 document.
    pub fn parse(bytes: &[u8]) -> Result<Self, ModelError> {
        Self::parse_with(bytes, false)
    }

    /// Like [`Self::parse`], but also accepts short API versions such as `1`
    /// and `1.0` (see [`ApiVersion::parse_lenient`]).
    ///
    /// # Errors
    ///
    /// Returns an error when the JSON is not a schema version 1 document.
    pub fn parse_lenient(bytes: &[u8]) -> Result<Self, ModelError> {
        Self::parse_with(bytes, true)
    }

    fn parse_with(bytes: &[u8], lenient: bool) -> Result<Self, ModelError> {
        let wire: WellKnownWire = serde_json::from_slice(bytes)?;
        if wire.schema_version != 1 {
            return Err(ModelError::Invalid(
                "well-known schemaVersion must be 1".into(),
            ));
        }
        if wire.endpoints.is_empty() {
            return Err(ModelError::Invalid(
                "well-known endpoints must not be empty".into(),
            ));
        }
        let mut endpoints = Vec::with_capacity(wire.endpoints.len());
        for endpoint in wire.endpoints {
            endpoints.push(parse_endpoint(endpoint, lenient)?);
        }
        Ok(Self { endpoints })
    }

    /// Endpoints in document order.
    #[must_use]
    pub fn endpoints(&self) -> &[WellKnownEndpoint] {
        &self.endpoints
    }

    /// Order endpoints for connection attempts.
    ///
    /// # Errors
    ///
    /// Returns an error when none advertise a supported version.
    pub fn select(&self, supported: &SupportedApi) -> Result<Vec<SelectedEndpoint>, ModelError> {
        let view: Vec<VersionedEndpoint> = self
            .endpoints
            .iter()
            .map(|endpoint| VersionedEndpoint {
                root_url: endpoint.url.clone(),
                versions: endpoint.versions.clone(),
                priority: endpoint.priority,
            })
            .collect();
        select_endpoints(&view, supported)
    }
}

impl DiscoveryInfo {
    /// Parse a successful `/discovery` body.
    ///
    /// An empty array is rejected. The specification requires `404` with a TEA
    /// error body when the identifier does not resolve, not `200 []`.
    ///
    /// # Errors
    ///
    /// Returns an error when the body is not a non-empty array of discovery objects.
    pub fn parse_list(bytes: &[u8]) -> Result<Vec<Self>, ModelError> {
        Self::parse_list_with(bytes, false)
    }

    /// Like [`Self::parse_list`], but also accepts short API versions such as
    /// `1` and `1.0` (see [`ApiVersion::parse_lenient`]).
    ///
    /// # Errors
    ///
    /// Returns an error when the body is not a non-empty array of discovery objects.
    pub fn parse_list_lenient(bytes: &[u8]) -> Result<Vec<Self>, ModelError> {
        Self::parse_list_with(bytes, true)
    }

    fn parse_list_with(bytes: &[u8], lenient: bool) -> Result<Vec<Self>, ModelError> {
        let wire: Vec<DiscoveryWire> = serde_json::from_slice(bytes)?;
        if wire.is_empty() {
            return Err(ModelError::Invalid(
                "discovery success body must be a non-empty array".into(),
            ));
        }
        wire.into_iter()
            .map(|item| parse_discovery(item, lenient))
            .collect()
    }

    /// Order this release's servers for later API calls.
    ///
    /// # Errors
    ///
    /// Returns an error when none advertise a supported version.
    pub fn select_server(
        &self,
        supported: &SupportedApi,
    ) -> Result<Vec<SelectedEndpoint>, ModelError> {
        let view: Vec<VersionedEndpoint> = self
            .servers
            .iter()
            .map(|server| VersionedEndpoint {
                root_url: server.root_url.clone(),
                versions: server.versions.clone(),
                priority: server.priority,
            })
            .collect();
        select_endpoints(&view, supported)
    }
}

fn parse_endpoint(endpoint: EndpointWire, lenient: bool) -> Result<WellKnownEndpoint, ModelError> {
    validate_base(&endpoint.url)?;
    Ok(WellKnownEndpoint {
        url: endpoint.url,
        versions: parse_versions(endpoint.versions, lenient)?,
        priority: parse_priority(endpoint.priority)?,
    })
}

fn parse_discovery(item: DiscoveryWire, lenient: bool) -> Result<DiscoveryInfo, ModelError> {
    if item.servers.is_empty() {
        return Err(ModelError::Invalid(
            "discovery servers must not be empty".into(),
        ));
    }
    let mut servers = Vec::with_capacity(item.servers.len());
    for server in item.servers {
        validate_base(&server.root_url)?;
        servers.push(DiscoveryServer {
            root_url: server.root_url,
            versions: parse_versions(server.versions, lenient)?,
            priority: parse_priority(server.priority)?,
        });
    }
    Ok(DiscoveryInfo {
        product_release: ProductReleaseId::parse(&item.product_release_uuid)?,
        servers,
    })
}

fn parse_versions(values: Vec<String>, lenient: bool) -> Result<Vec<ApiVersion>, ModelError> {
    if values.is_empty() {
        return Err(ModelError::Invalid("versions must not be empty".into()));
    }
    values
        .iter()
        .map(|value| {
            if lenient {
                ApiVersion::parse_lenient(value)
            } else {
                ApiVersion::parse(value)
            }
        })
        .collect()
}

fn parse_priority(value: Option<f64>) -> Result<OrderedPriority, ModelError> {
    match value {
        None => Ok(OrderedPriority::default_priority()),
        Some(priority) => OrderedPriority::parse(priority),
    }
}

fn validate_base(text: &str) -> Result<Url, ModelError> {
    parse_api_base(text).map_err(|err: PolicyError| ModelError::Invalid(err.to_string()))
}

/// Decide whether a `404` body is a TEA error response.
///
/// Recognition follows the discovery document: `Content-Type` is
/// `application/json` with optional parameters, and the body is a JSON object
/// with a string `error` property. Unknown `error` values still count. This is
/// intentionally wider than `additionalProperties: false`, because the prose
/// says a later error value must not become a failover trigger.
#[must_use]
pub fn tea_error_code(content_type: Option<&str>, body: &[u8]) -> Option<String> {
    if !is_json_content_type(content_type) {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let error = value.get("error")?.as_str()?;
    Some(error.to_owned())
}

/// Whether a media type is `application/json`, ignoring parameters and case.
#[must_use]
pub fn is_json_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    let essence = content_type.split(';').next().unwrap_or("").trim();
    essence.eq_ignore_ascii_case("application/json")
}

/// Append one query parameter, encoding it a single time.
///
/// # Errors
///
/// Returns an error when `prefix` is not a valid URL.
pub fn with_query(prefix: &str, name: &str, value: &str) -> Result<Url, ModelError> {
    let mut url = Url::parse(prefix)
        .map_err(|err| ModelError::Invalid(format!("cannot build query URL: {err}")))?;
    url.query_pairs_mut().clear().append_pair(name, value);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_normative_well_known_example() {
        let json = br#"{
          "schemaVersion": 1,
          "endpoints": [
            {"url": "https://api.example.com", "versions": ["1.0.0"], "priority": 1},
            {"url": "https://api2.example.com/mytea", "versions": ["1.0.0"], "priority": 0.5}
          ]
        }"#;
        let doc = WellKnownDocument::parse(json).unwrap();
        let selected = doc.select(&SupportedApi::tea_1_0_0()).unwrap();
        assert_eq!(selected[0].root_url, "https://api.example.com");
        assert_eq!(selected[1].priority.get(), 0.5);
    }

    #[test]
    fn omitted_priority_is_one_and_http_base_parses() {
        let json = br#"{
          "schemaVersion": 1,
          "endpoints": [{"url": "http://localhost:8080/tea", "versions": ["1.0.0"]}]
        }"#;
        let doc = WellKnownDocument::parse(json).unwrap();
        assert_eq!(doc.endpoints()[0].priority.get(), 1.0);
    }

    #[test]
    fn empty_discovery_array_is_rejected() {
        assert!(DiscoveryInfo::parse_list(b"[]").is_err());
    }

    #[test]
    fn tea_error_recognition_keeps_unknown_codes() {
        assert_eq!(
            tea_error_code(
                Some("application/json; charset=utf-8"),
                br#"{"error":"FUTURE","extra":1}"#
            ),
            Some("FUTURE".into())
        );
        assert!(tea_error_code(Some("application/json"), br#"{"message":"Not Found"}"#).is_none());
        assert!(tea_error_code(Some("text/html"), br#"{"error":"OBJECT_UNKNOWN"}"#).is_none());
    }
}
