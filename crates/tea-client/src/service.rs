//! Typed TEA consumer operations.

use serde::de::DeserializeOwned;
use tea_core::{
    is_json_content_type, parse_artifact, parse_collection, parse_component_release,
    parse_product_release, versioned_api_prefix, Artifact, Checksum, Cle, Collection, Component,
    ComponentId, ComponentRelease, ComponentReleaseId, ComponentReleaseWithCollection,
    DiscoveryInfo, IntegrityOutcome, Page, Product, ProductId, ProductRelease, ProductReleaseId,
    ProductReleaseWithCollection,
};
use url::Url;

use crate::error::ClientError;
use crate::transport::{header_str, map_api_error, operation_url, Call, CONTENT_TYPE_HEADER};
use crate::Client;

/// A versioned TEA API base.
///
/// Paths are OpenAPI paths appended to `{rootUrl}/v{advertised-version}/`.
#[derive(Clone)]
pub struct Service {
    client: Client,
    root: String,
    version: String,
    prefix: Url,
}

/// Bytes of an artifact or signature after HTTP content coding is removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedArtifact {
    /// Decoded bytes.
    pub bytes: Vec<u8>,
    /// Response `Content-Type`.
    pub content_type: Option<String>,
    /// HTTP content coding that was removed, when one was declared.
    pub content_encoding: Option<String>,
    /// `ETag` as received. This is not a checksum.
    pub etag: Option<String>,
    /// Whether the tag is strong. A weak tag is not byte-identity proof.
    pub strong_etag: bool,
    /// `Cache-Control` as received. This client does not store a cache.
    pub cache_control: Option<String>,
    /// `Content-Location` as received.
    pub content_location: Option<String>,
    /// `Vary` as received.
    pub vary: Option<String>,
    /// `Repr-Digest` as received. It covers the HTTP representation, not the artifact checksum.
    pub repr_digest: Option<String>,
    /// Final URL after redirects.
    pub final_url: String,
    /// Checksum report. [`IntegrityOutcome::Verified`] means every declared checksum matched.
    pub integrity: tea_core::IntegrityReport,
}

/// Artifact download result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactFetch {
    /// A response body.
    Body(DownloadedArtifact),
    /// Conditional GET found the selected representation unchanged.
    NotModified {
        /// Validator from the 304 response.
        etag: Option<String>,
        /// Cache policy.
        cache_control: Option<String>,
        /// Vary header.
        vary: Option<String>,
    },
}

/// CLE projection result. `NotImplemented` is not a missing parent object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleFetch {
    /// Lifecycle projection.
    Events(Cle),
    /// The server does not implement CLE for this object.
    NotImplemented {
        /// Optional diagnostic.
        message: Option<String>,
    },
}

/// Detached signature fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureFetch {
    /// Signature bytes. TEA does not define a checksum for the signature itself.
    Present(Box<DownloadedArtifact>),
    /// The revision exists and this format has no published signature.
    NotPublished,
    /// Conditional GET matched.
    NotModified {
        /// Validator.
        etag: Option<String>,
    },
}

impl Service {
    pub(crate) fn from_parts(
        client: Client,
        root: Url,
        version: String,
    ) -> Result<Self, ClientError> {
        let prefix = versioned_api_prefix(root.as_str().trim_end_matches('/'), &version)?;
        let prefix = Url::parse(&prefix).map_err(|_| {
            ClientError::Model(tea_core::ModelError::Invalid(
                "versioned API prefix is not a URL".into(),
            ))
        })?;
        Ok(Self {
            client,
            root: root.as_str().trim_end_matches('/').to_owned(),
            version,
            prefix,
        })
    }

    /// API root without the version segment.
    #[must_use]
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Advertised version string used in the path.
    #[must_use]
    pub fn api_version(&self) -> &str {
        &self.version
    }

    /// Resolve a TEI or PURL on this already-known API base.
    ///
    /// Exactly one of the arguments must be set.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Unresolved`] for a TEA 404, which does not mean failover.
    pub async fn discover_identifier(
        &self,
        tei: Option<&str>,
        purl: Option<&str>,
    ) -> Result<Vec<DiscoveryInfo>, ClientError> {
        match (tei, purl) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(ClientError::Model(tea_core::ModelError::Invalid(
                    "exactly one of tei or purl is required".into(),
                )));
            }
            _ => {}
        }
        let mut query = Vec::new();
        if let Some(tei) = tei {
            query.push(("tei", tei));
        }
        if let Some(purl) = purl {
            query.push(("purl", purl));
        }
        let response = self.get_raw("discovery", &query, false).await?;
        if response.status.as_u16() == 404 {
            if let Some(error) = tea_core::tea_error_code(
                header_str(&response.headers, CONTENT_TYPE_HEADER),
                &response.body,
            ) {
                let message = tea_core::TeaErrorBody::parse(&response.body)
                    .ok()
                    .and_then(|body| body.message);
                return Err(ClientError::Unresolved { error, message });
            }
        }
        if !response.status.is_success() {
            return Err(map_api_error(&response));
        }
        require_json(&response)?;
        Ok(DiscoveryInfo::parse_list(&response.body)?)
    }

    /// `GET /products`.
    ///
    /// # Errors
    ///
    /// Returns an error on transport failure or a non-success status.
    pub async fn query_products(
        &self,
        query: &tea_core::ListParams<tea_core::enums::ProductSort>,
    ) -> Result<Page<Product>, ClientError> {
        self.page("products", query, |sort| sort.as_str()).await
    }

    /// `GET /product/{uuid}`.
    pub async fn get_product(&self, id: ProductId) -> Result<Product, ClientError> {
        self.get_json(&format!("product/{id}"), &[]).await
    }

    /// `GET /product/{uuid}/releases`.
    pub async fn list_product_releases(
        &self,
        id: ProductId,
        query: &tea_core::ListParams<tea_core::enums::ReleaseSort>,
    ) -> Result<Page<ProductRelease>, ClientError> {
        self.page(&format!("product/{id}/releases"), query, |sort| {
            sort.as_str()
        })
        .await
    }

    /// `GET /product/{uuid}/cle`.
    pub async fn get_product_cle(&self, id: ProductId) -> Result<CleFetch, ClientError> {
        self.cle(&format!("product/{id}/cle")).await
    }

    /// `GET /productReleases`.
    pub async fn query_product_releases(
        &self,
        query: &tea_core::ListParams<tea_core::enums::ReleaseSort>,
    ) -> Result<Page<ProductRelease>, ClientError> {
        self.page("productReleases", query, |sort| sort.as_str())
            .await
    }

    /// `GET /productRelease/{uuid}`.
    pub async fn get_product_release(
        &self,
        id: ProductReleaseId,
    ) -> Result<ProductReleaseWithCollection, ClientError> {
        let bytes = self.get_bytes(&format!("productRelease/{id}"), &[]).await?;
        Ok(parse_product_release(&bytes)?)
    }

    /// `GET /productRelease/{uuid}/cle`.
    pub async fn get_product_release_cle(
        &self,
        id: ProductReleaseId,
    ) -> Result<CleFetch, ClientError> {
        self.cle(&format!("productRelease/{id}/cle")).await
    }

    /// `GET /productRelease/{uuid}/collections`.
    pub async fn list_product_release_collections(
        &self,
        id: ProductReleaseId,
        query: &tea_core::ListParams<tea_core::enums::CollectionSort>,
    ) -> Result<Page<Collection>, ClientError> {
        let page: Page<Collection> = self
            .page(&format!("productRelease/{id}/collections"), query, |sort| {
                sort.as_str()
            })
            .await?;
        for collection in &page.results {
            collection.validate()?;
        }
        Ok(page)
    }

    /// `GET /productRelease/{uuid}/collection/latest`.
    pub async fn get_latest_product_release_collection(
        &self,
        id: ProductReleaseId,
    ) -> Result<Collection, ClientError> {
        let bytes = self
            .get_bytes(&format!("productRelease/{id}/collection/latest"), &[])
            .await?;
        Ok(parse_collection(&bytes)?)
    }

    /// `GET /productRelease/{uuid}/collection/{version}`.
    pub async fn get_product_release_collection(
        &self,
        id: ProductReleaseId,
        version: u64,
    ) -> Result<Collection, ClientError> {
        let bytes = self
            .get_bytes(&format!("productRelease/{id}/collection/{version}"), &[])
            .await?;
        Ok(parse_collection(&bytes)?)
    }

    /// `GET /components`.
    pub async fn query_components(
        &self,
        query: &tea_core::ListParams<tea_core::enums::ComponentSort>,
    ) -> Result<Page<Component>, ClientError> {
        self.page("components", query, |sort| sort.as_str()).await
    }

    /// `GET /component/{uuid}`.
    pub async fn get_component(&self, id: ComponentId) -> Result<Component, ClientError> {
        self.get_json(&format!("component/{id}"), &[]).await
    }

    /// `GET /component/{uuid}/releases`.
    pub async fn list_component_releases(
        &self,
        id: ComponentId,
        query: &tea_core::ListParams<tea_core::enums::ReleaseSort>,
    ) -> Result<Page<ComponentRelease>, ClientError> {
        self.page(&format!("component/{id}/releases"), query, |sort| {
            sort.as_str()
        })
        .await
    }

    /// `GET /component/{uuid}/cle`.
    pub async fn get_component_cle(&self, id: ComponentId) -> Result<CleFetch, ClientError> {
        self.cle(&format!("component/{id}/cle")).await
    }

    /// `GET /componentReleases`.
    pub async fn query_component_releases(
        &self,
        query: &tea_core::ListParams<tea_core::enums::ReleaseSort>,
    ) -> Result<Page<ComponentRelease>, ClientError> {
        self.page("componentReleases", query, |sort| sort.as_str())
            .await
    }

    /// `GET /componentRelease/{uuid}`.
    pub async fn get_component_release(
        &self,
        id: ComponentReleaseId,
    ) -> Result<ComponentReleaseWithCollection, ClientError> {
        let bytes = self
            .get_bytes(&format!("componentRelease/{id}"), &[])
            .await?;
        Ok(parse_component_release(&bytes)?)
    }

    /// `GET /componentRelease/{uuid}/cle`.
    pub async fn get_component_release_cle(
        &self,
        id: ComponentReleaseId,
    ) -> Result<CleFetch, ClientError> {
        self.cle(&format!("componentRelease/{id}/cle")).await
    }

    /// `GET /componentRelease/{uuid}/collections`.
    pub async fn list_component_release_collections(
        &self,
        id: ComponentReleaseId,
        query: &tea_core::ListParams<tea_core::enums::CollectionSort>,
    ) -> Result<Page<Collection>, ClientError> {
        let page: Page<Collection> = self
            .page(
                &format!("componentRelease/{id}/collections"),
                query,
                |sort| sort.as_str(),
            )
            .await?;
        for collection in &page.results {
            collection.validate()?;
        }
        Ok(page)
    }

    /// `GET /componentRelease/{uuid}/collection/latest`.
    pub async fn get_latest_component_release_collection(
        &self,
        id: ComponentReleaseId,
    ) -> Result<Collection, ClientError> {
        let bytes = self
            .get_bytes(&format!("componentRelease/{id}/collection/latest"), &[])
            .await?;
        Ok(parse_collection(&bytes)?)
    }

    /// `GET /componentRelease/{uuid}/collection/{version}`.
    pub async fn get_component_release_collection(
        &self,
        id: ComponentReleaseId,
        version: u64,
    ) -> Result<Collection, ClientError> {
        let bytes = self
            .get_bytes(&format!("componentRelease/{id}/collection/{version}"), &[])
            .await?;
        Ok(parse_collection(&bytes)?)
    }

    /// `GET /artifact/{uuid}/latest`.
    pub async fn get_latest_artifact(
        &self,
        id: tea_core::ArtifactId,
    ) -> Result<Artifact, ClientError> {
        let bytes = self
            .get_bytes(&format!("artifact/{id}/latest"), &[])
            .await?;
        Ok(parse_artifact(&bytes)?)
    }

    /// `GET /artifact/{uuid}/{version}`.
    pub async fn get_artifact(
        &self,
        id: tea_core::ArtifactId,
        version: u64,
    ) -> Result<Artifact, ClientError> {
        let bytes = self
            .get_bytes(&format!("artifact/{id}/{version}"), &[])
            .await?;
        Ok(parse_artifact(&bytes)?)
    }

    /// Download one format of an artifact revision.
    ///
    /// When the format has `url`, that external URL is fetched and the TEA
    /// access token is not sent. Otherwise the versioned download endpoint is
    /// used. Declared checksums are verified over the bytes left after HTTP
    /// content coding is removed.
    pub async fn download_format(
        &self,
        artifact: &Artifact,
        media_type: &str,
        if_none_match: Option<&str>,
    ) -> Result<ArtifactFetch, ClientError> {
        let format = artifact.format(media_type).ok_or_else(|| {
            ClientError::Model(tea_core::ModelError::Invalid(format!(
                "artifact has no format `{media_type}`"
            )))
        })?;
        format.validate()?;
        let checksums = format.checksums.clone().unwrap_or_default();
        if let Some(external) = &format.url {
            let url = Url::parse(external).map_err(|_| {
                ClientError::Model(tea_core::ModelError::Invalid(
                    "external artifact URL".into(),
                ))
            })?;
            return self
                .download_url(url, None, if_none_match, &checksums, false)
                .await;
        }
        self.download_api(
            &format!("artifact/{}/{}/download", artifact.uuid, artifact.version),
            Some(media_type),
            if_none_match,
            &checksums,
        )
        .await
    }

    /// `GET /artifact/{uuid}/latest/download`.
    pub async fn download_latest_artifact(
        &self,
        id: tea_core::ArtifactId,
        media_type: Option<&str>,
        checksums: &[Checksum],
        if_none_match: Option<&str>,
    ) -> Result<ArtifactFetch, ClientError> {
        self.download_api(
            &format!("artifact/{id}/latest/download"),
            media_type,
            if_none_match,
            checksums,
        )
        .await
    }

    /// `GET /artifact/{uuid}/{version}/download`.
    pub async fn download_artifact(
        &self,
        id: tea_core::ArtifactId,
        version: u64,
        media_type: Option<&str>,
        checksums: &[Checksum],
        if_none_match: Option<&str>,
    ) -> Result<ArtifactFetch, ClientError> {
        self.download_api(
            &format!("artifact/{id}/{version}/download"),
            media_type,
            if_none_match,
            checksums,
        )
        .await
    }

    /// Download a detached signature for one content format.
    ///
    /// `media_type` selects the artifact format, not the signature's own media type.
    /// An external `signatureUrl` is fetched without a TEA access token.
    pub async fn download_signature_for_format(
        &self,
        artifact: &Artifact,
        media_type: &str,
        if_none_match: Option<&str>,
    ) -> Result<SignatureFetch, ClientError> {
        let format = artifact.format(media_type).ok_or_else(|| {
            ClientError::Model(tea_core::ModelError::Invalid(format!(
                "artifact has no format `{media_type}`"
            )))
        })?;
        if let Some(external) = &format.signature_url {
            let url = Url::parse(external).map_err(|_| {
                ClientError::Model(tea_core::ModelError::Invalid(
                    "external artifact URL".into(),
                ))
            })?;
            return match self
                .download_url(url, None, if_none_match, &[], false)
                .await?
            {
                ArtifactFetch::Body(body) => Ok(SignatureFetch::Present(Box::new(body))),
                ArtifactFetch::NotModified { etag, .. } => Ok(SignatureFetch::NotModified { etag }),
            };
        }
        self.signature_api(
            &format!(
                "artifact/{}/{}/signature/download",
                artifact.uuid, artifact.version
            ),
            Some(media_type),
            if_none_match,
        )
        .await
    }

    /// `GET /artifact/{uuid}/{version}/signature/download`.
    pub async fn download_signature(
        &self,
        id: tea_core::ArtifactId,
        version: u64,
        media_type: Option<&str>,
        if_none_match: Option<&str>,
    ) -> Result<SignatureFetch, ClientError> {
        self.signature_api(
            &format!("artifact/{id}/{version}/signature/download"),
            media_type,
            if_none_match,
        )
        .await
    }

    /// `GET /artifact/{uuid}/latest/signature/download`.
    pub async fn download_latest_signature(
        &self,
        id: tea_core::ArtifactId,
        media_type: Option<&str>,
        if_none_match: Option<&str>,
    ) -> Result<SignatureFetch, ClientError> {
        self.signature_api(
            &format!("artifact/{id}/latest/signature/download"),
            media_type,
            if_none_match,
        )
        .await
    }

    /// `HEAD` the versioned artifact download and return the response headers without a body.
    pub async fn head_artifact(
        &self,
        id: tea_core::ArtifactId,
        version: u64,
        media_type: Option<&str>,
    ) -> Result<std::collections::HashMap<String, String>, ClientError> {
        let mut query = Vec::new();
        if let Some(media_type) = media_type {
            query.push(("mediaType", media_type));
        }
        let url = operation_url(
            &self.prefix,
            &format!("artifact/{id}/{version}/download"),
            &query,
        )?;
        let response = self
            .client
            .inner
            .send(Call {
                method: reqwest::Method::HEAD,
                url,
                api_prefix: Some(self.prefix.clone()),
                artifact: true,
                body: None,
                content_type: None,
                basic_auth: false,
                if_none_match: None,
                max_body: 0,
            })
            .await?;
        if !response.status.is_success() && response.status.as_u16() != 304 {
            return Err(map_api_error(&response));
        }
        let mut headers = std::collections::HashMap::new();
        for (name, value) in response.headers.iter() {
            if name == reqwest::header::AUTHORIZATION {
                continue;
            }
            if let Ok(text) = value.to_str() {
                headers.insert(name.to_string(), text.to_owned());
            }
        }
        Ok(headers)
    }

    async fn page<T, S, F>(
        &self,
        path: &str,
        query: &tea_core::ListParams<S>,
        sort_name: F,
    ) -> Result<Page<T>, ClientError>
    where
        T: DeserializeOwned,
        F: Fn(&S) -> &'static str,
    {
        query.validate()?;
        let mut pairs: Vec<(String, String)> = Vec::new();
        if let Some(size) = query.page_size {
            pairs.push(("pageSize".into(), size.to_string()));
        }
        if let Some(token) = &query.page_token {
            pairs.push(("pageToken".into(), token.clone()));
        }
        if let Some(field) = &query.sort_field {
            pairs.push(("sortField".into(), sort_name(field).to_owned()));
        }
        if let Some(order) = query.sort_order {
            pairs.push(("sortOrder".into(), order.as_str().to_owned()));
        }
        if let Some(id_type) = &query.id_type {
            pairs.push(("idType".into(), id_type.as_str().to_owned()));
        }
        if let Some(id_value) = &query.id_value {
            pairs.push(("idValue".into(), id_value.clone()));
        }
        let borrowed: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        self.get_json(path, &borrowed).await
    }

    async fn cle(&self, path: &str) -> Result<CleFetch, ClientError> {
        let response = self.get_raw(path, &[], false).await?;
        if response.status.as_u16() == 404 {
            if let Some(code) = tea_core::tea_error_code(
                header_str(&response.headers, CONTENT_TYPE_HEADER),
                &response.body,
            ) {
                if code == "NOT_IMPLEMENTED" {
                    let message = tea_core::TeaErrorBody::parse(&response.body)
                        .ok()
                        .and_then(|body| body.message);
                    return Ok(CleFetch::NotImplemented { message });
                }
            }
            return Err(map_api_error(&response));
        }
        if !response.status.is_success() {
            return Err(map_api_error(&response));
        }
        require_json(&response)?;
        Ok(CleFetch::Events(serde_json::from_slice(&response.body)?))
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ClientError> {
        let bytes = self.get_bytes(path, query).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn get_bytes(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<u8>, ClientError> {
        let response = self.get_raw(path, query, false).await?;
        if !response.status.is_success() {
            return Err(map_api_error(&response));
        }
        require_json(&response)?;
        Ok(response.body.to_vec())
    }

    async fn get_raw(
        &self,
        path: &str,
        query: &[(&str, &str)],
        artifact: bool,
    ) -> Result<crate::transport::RawResponse, ClientError> {
        let url = operation_url(&self.prefix, path, query)?;
        self.client
            .inner
            .send(Call {
                method: reqwest::Method::GET,
                url,
                api_prefix: Some(self.prefix.clone()),
                artifact,
                body: None,
                content_type: None,
                basic_auth: false,
                if_none_match: None,
                max_body: if artifact {
                    self.client.inner.limits.max_artifact_bytes
                } else {
                    self.client.inner.limits.max_json_bytes
                },
            })
            .await
    }

    async fn download_api(
        &self,
        path: &str,
        media_type: Option<&str>,
        if_none_match: Option<&str>,
        checksums: &[Checksum],
    ) -> Result<ArtifactFetch, ClientError> {
        let mut query = Vec::new();
        if let Some(media_type) = media_type {
            query.push(("mediaType", media_type));
        }
        let url = operation_url(&self.prefix, path, &query)?;
        self.download_url(
            url,
            Some(self.prefix.clone()),
            if_none_match,
            checksums,
            true,
        )
        .await
    }

    async fn download_url(
        &self,
        url: Url,
        api_prefix: Option<Url>,
        if_none_match: Option<&str>,
        checksums: &[Checksum],
        under_api: bool,
    ) -> Result<ArtifactFetch, ClientError> {
        let response = self
            .client
            .inner
            .send(Call {
                method: reqwest::Method::GET,
                url,
                api_prefix: if under_api { api_prefix } else { None },
                artifact: true,
                body: None,
                content_type: None,
                basic_auth: false,
                if_none_match: if_none_match.map(str::to_owned),
                max_body: self.client.inner.limits.max_artifact_bytes,
            })
            .await?;
        if response.status.as_u16() == 304 {
            return Ok(ArtifactFetch::NotModified {
                etag: header_string(&response, "etag"),
                cache_control: header_string(&response, "cache-control"),
                vary: header_string(&response, "vary"),
            });
        }
        if !response.status.is_success() {
            return Err(map_api_error(&response));
        }
        let encoding = header_string(&response, "content-encoding");
        let decoded = if response.content_decoded {
            response.body.to_vec()
        } else {
            tea_core::checksum::decode_content_coding(
                &response.body,
                encoding.as_deref(),
                self.client.inner.limits.max_artifact_bytes,
            )?
        };
        let integrity = tea_core::verify_checksums(&decoded, checksums);
        if !matches!(
            integrity.outcome,
            IntegrityOutcome::Verified | IntegrityOutcome::NotDeclared
        ) {
            return Err(ClientError::Integrity {
                outcome: integrity.outcome.clone(),
                report: integrity,
            });
        }
        let etag = header_string(&response, "etag");
        let strong_etag = etag
            .as_ref()
            .is_some_and(|value| !value.trim().starts_with("W/"));
        Ok(ArtifactFetch::Body(DownloadedArtifact {
            bytes: decoded,
            content_type: header_string(&response, "content-type"),
            content_encoding: encoding,
            etag,
            strong_etag,
            cache_control: header_string(&response, "cache-control"),
            content_location: header_string(&response, "content-location"),
            vary: header_string(&response, "vary"),
            repr_digest: header_string(&response, "repr-digest"),
            final_url: response.final_url.to_string(),
            integrity,
        }))
    }

    async fn signature_api(
        &self,
        path: &str,
        media_type: Option<&str>,
        if_none_match: Option<&str>,
    ) -> Result<SignatureFetch, ClientError> {
        match self
            .download_api(path, media_type, if_none_match, &[])
            .await
        {
            Ok(ArtifactFetch::Body(body)) => Ok(SignatureFetch::Present(Box::new(body))),
            Ok(ArtifactFetch::NotModified { etag, .. }) => Ok(SignatureFetch::NotModified { etag }),
            Err(ClientError::Api {
                status: 404,
                tea_error: Some(code),
                ..
            }) if code == "SIGNATURE_NOT_FOUND" => Ok(SignatureFetch::NotPublished),
            Err(other) => Err(other),
        }
    }
}

fn require_json(response: &crate::transport::RawResponse) -> Result<(), ClientError> {
    if is_json_content_type(header_str(&response.headers, CONTENT_TYPE_HEADER)) {
        Ok(())
    } else {
        Err(ClientError::Model(tea_core::ModelError::Invalid(
            "expected application/json".into(),
        )))
    }
}

fn header_string(response: &crate::transport::RawResponse, name: &str) -> Option<String> {
    response
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
