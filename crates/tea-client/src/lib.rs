//! Async consumer client for the Transparency Exchange API 1.0.0.
//!
//! Transport uses rustls. Certificate verification stays enabled. Discovery
//! and artifact URLs are checked against [`libtea_core::DestinationPolicy`] before each
//! connection, including addresses returned by DNS.

#![forbid(unsafe_code)]

mod error;
mod service;
mod transport;

pub use error::ClientError;
pub use service::{ArtifactFetch, CleFetch, DownloadedArtifact, Service, SignatureFetch};
pub use transport::{ApiKey, Client, ClientBuilder, Limits};

use libtea_core::{tea_error_code, DiscoveryInfo, ProductReleaseId, Tei, WellKnownDocument};
use url::Url;

use crate::transport::{header_str, map_api_error, operation_url, Call, CONTENT_TYPE_HEADER};

/// A product release selected from `/discovery`, with the server and version to call next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRelease {
    /// Product release UUID.
    pub product_release: ProductReleaseId,
    /// API root without a version prefix.
    pub root_url: String,
    /// Exact advertised version string.
    pub api_version: String,
}

impl Client {
    /// Resolve a TEI through `.well-known/tea` and `/discovery`.
    ///
    /// A TEA 404 from `/discovery` stops the search and returns
    /// [`ClientError::Unresolved`]. It is not treated as a missing route and
    /// it is not evidence that no updates exist. `401` and `403` do not fail
    /// over to another endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the TEI cannot be resolved or discovery attempts
    /// are exhausted.
    pub async fn discover(&self, tei: &Tei) -> Result<Vec<ResolvedRelease>, ClientError> {
        let mut attempts = 0u32;
        let well_known_url = match &self.inner.well_known_override {
            Some(url) => url.clone(),
            None => Url::parse(&tei.well_known_url()).map_err(|_| {
                ClientError::Model(libtea_core::ModelError::Invalid(
                    "well-known URL is not valid".into(),
                ))
            })?,
        };
        attempts += 1;
        let document = match self.fetch_well_known(&well_known_url).await {
            Ok(document) => document,
            Err(error) => {
                tracing::debug!(attempts, "well-known discovery failed");
                return Err(match error {
                    ClientError::Transport(_) | ClientError::Timeout | ClientError::TooLarge => {
                        ClientError::DiscoveryFailed {
                            stage: "well-known",
                            attempts,
                        }
                    }
                    other => other,
                });
            }
        };
        let candidates = document.select(&self.inner.supported)?;
        let mut last_stage_error = ClientError::DiscoveryFailed {
            stage: "api",
            attempts,
        };
        for candidate in candidates {
            if attempts >= self.inner.limits.max_discovery_attempts {
                return Err(ClientError::DiscoveryFailed {
                    stage: "api",
                    attempts,
                });
            }
            attempts += 1;
            let root = Url::parse(&candidate.root_url).map_err(|_| {
                ClientError::Model(libtea_core::ModelError::Invalid(
                    "discovery root URL".into(),
                ))
            })?;
            self.inner.policy.check_url(&root)?;
            if self.inner.allow_discovered_credentials {
                self.authorize_origin(&root)?;
            }
            let prefix = Url::parse(&libtea_core::versioned_api_prefix(
                root.as_str().trim_end_matches('/'),
                &candidate.version,
            )?)
            .map_err(|_| {
                ClientError::Model(libtea_core::ModelError::Invalid(
                    "versioned discovery URL".into(),
                ))
            })?;
            let discovery_url = operation_url(&prefix, "discovery", &[("tei", tei.as_str())])?;
            let response = self
                .inner
                .send(Call {
                    method: reqwest::Method::GET,
                    url: discovery_url,
                    api_prefix: Some(prefix.clone()),
                    artifact: false,
                    body: None,
                    content_type: None,
                    basic_auth: false,
                    if_none_match: None,
                    max_body: self.inner.limits.max_json_bytes,
                })
                .await;
            let response = match response {
                Ok(response) => response,
                Err(
                    error @ (ClientError::AuthenticationRequired
                    | ClientError::Forbidden
                    | ClientError::Token { .. }),
                ) => return Err(error),
                Err(error) => {
                    last_stage_error = error;
                    self.inner.backoff(attempts).await;
                    continue;
                }
            };
            if response.status.as_u16() == 401 || response.status.as_u16() == 403 {
                return Err(map_api_error(&response));
            }
            if response.status.as_u16() == 404 {
                if let Some(error) = tea_error_code(
                    header_str(&response.headers, CONTENT_TYPE_HEADER),
                    &response.body,
                ) {
                    let message = libtea_core::TeaErrorBody::parse(&response.body)
                        .ok()
                        .and_then(|body| body.message);
                    return Err(ClientError::Unresolved { error, message });
                }
                last_stage_error = map_api_error(&response);
                self.inner.backoff(attempts).await;
                continue;
            }
            if !response.status.is_success() {
                last_stage_error = map_api_error(&response);
                self.inner.backoff(attempts).await;
                continue;
            }
            let releases = match DiscoveryInfo::parse_list(&response.body) {
                Ok(releases) => releases,
                Err(error) => {
                    last_stage_error = ClientError::Model(error);
                    self.inner.backoff(attempts).await;
                    continue;
                }
            };
            let mut resolved = Vec::with_capacity(releases.len());
            for release in releases {
                let server = release.select_server(&self.inner.supported)?;
                let chosen = server.into_iter().next().ok_or_else(|| {
                    ClientError::Model(libtea_core::ModelError::Invalid(
                        "discovery returned no usable server".into(),
                    ))
                })?;
                resolved.push(ResolvedRelease {
                    product_release: release.product_release,
                    root_url: chosen.root_url,
                    api_version: chosen.version,
                });
            }
            return Ok(resolved);
        }
        Err(last_stage_error)
    }

    /// Open a service for a resolved release.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected base URL is not allowed.
    pub fn open(&self, release: &ResolvedRelease) -> Result<Service, ClientError> {
        self.service(&release.root_url, &release.api_version)
    }

    async fn fetch_well_known(&self, url: &Url) -> Result<WellKnownDocument, ClientError> {
        self.inner.policy.check_url(url)?;
        let response = self
            .inner
            .send(Call {
                method: reqwest::Method::GET,
                url: url.clone(),
                api_prefix: None,
                artifact: false,
                body: None,
                content_type: None,
                basic_auth: false,
                if_none_match: None,
                max_body: self.inner.limits.max_discovery_bytes,
            })
            .await?;
        if !response.status.is_success() {
            return Err(map_api_error(&response));
        }
        Ok(WellKnownDocument::parse(&response.body)?)
    }
}
