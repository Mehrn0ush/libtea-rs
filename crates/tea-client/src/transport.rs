//! HTTP transport with manual redirects, body limits, and credential scoping.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use cdx_tea_core::checksum::ContentDecoder;
use cdx_tea_core::{
    classify_ip, origin_key, parse_api_base, tea_error_code, ApiVersion, DestinationPolicy,
    SupportedApi, TeaErrorBody, TokenResponse,
};
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE, LOCATION, WWW_AUTHENTICATE};
use reqwest::{Method, StatusCode};
use tokio::sync::Semaphore;
use url::Url;

use crate::error::ClientError;

/// Bounds for untrusted TEA responses.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum `.well-known/tea` document size.
    pub max_discovery_bytes: usize,
    /// Maximum JSON API response size.
    pub max_json_bytes: usize,
    /// Maximum artifact or signature size, before and after content decoding.
    pub max_artifact_bytes: usize,
    /// Maximum redirects for one operation.
    pub max_redirects: usize,
    /// Overall request timeout.
    pub request_timeout: Duration,
    /// TCP connect timeout.
    pub connect_timeout: Duration,
    /// Extra attempts for idempotent GET transport failures and HTTP 503.
    pub max_retries: u32,
    /// Total discovery attempts across the well-known fetch and API candidates.
    pub max_discovery_attempts: u32,
    /// Base delay between discovery attempts. Zero disables the delay.
    pub initial_backoff: Duration,
    /// Maximum concurrent HTTP operations.
    pub max_in_flight: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_discovery_bytes: 1024 * 1024,
            max_json_bytes: 8 * 1024 * 1024,
            max_artifact_bytes: 256 * 1024 * 1024,
            max_redirects: 5,
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_retries: 2,
            max_discovery_attempts: 4,
            initial_backoff: Duration::from_millis(200),
            max_in_flight: 8,
        }
    }
}

/// API key identifier and secret. The secret is redacted in `Debug`.
pub struct ApiKey {
    id: String,
    secret: String,
}

impl ApiKey {
    /// Build a key pair.
    #[must_use]
    pub fn new(id: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            secret: secret.into(),
        }
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKey")
            .field("id", &self.id)
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
struct StoredToken {
    value: String,
    expires_at: Option<Instant>,
}

impl std::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredToken")
            .field("value", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub(crate) struct ClientInner {
    http: reqwest::Client,
    pub(crate) limits: Limits,
    pub(crate) policy: DestinationPolicy,
    pub(crate) supported: SupportedApi,
    credentials: Option<ApiKey>,
    /// Origins this client may send the API key to.
    credential_origins: Mutex<BTreeSet<String>>,
    pub(crate) allow_discovered_credentials: bool,
    tokens: Mutex<std::collections::HashMap<String, StoredToken>>,
    semaphore: Arc<Semaphore>,
    pub(crate) well_known_override: Option<Url>,
}

/// TEA consumer client.
///
/// Cloning shares the connection pool, token cache, and limits.
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

/// Builder for [`Client`].
pub struct ClientBuilder {
    limits: Limits,
    policy: DestinationPolicy,
    supported: SupportedApi,
    credentials: Option<ApiKey>,
    allow_discovered_credentials: bool,
    well_known_override: Option<Url>,
    roots_pem: Vec<u8>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            policy: DestinationPolicy::public_https(),
            supported: SupportedApi::tea_1_0_0(),
            credentials: None,
            allow_discovered_credentials: false,
            well_known_override: None,
            roots_pem: Vec::new(),
        }
    }
}

impl ClientBuilder {
    /// Create a builder with production defaults: HTTPS, public addresses, TEA 1.0.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace resource limits.
    #[must_use]
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Replace the destination policy.
    ///
    /// The default rejects cleartext HTTP, loopback, link-local, and private addresses.
    #[must_use]
    pub fn destination_policy(mut self, policy: DestinationPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Versions this client is willing to speak. Build metadata on the server is ignored.
    #[must_use]
    pub fn supported_versions(mut self, supported: SupportedApi) -> Self {
        self.supported = supported;
        self
    }

    /// API key used at `POST /token` for origins the caller has explicitly selected.
    ///
    /// Discovery does not send this key to endpoints learned from `.well-known/tea`
    /// unless [`Self::allow_discovered_credentials`] is set. A [`crate::Service`]
    /// built from a caller-supplied base URL may use the key at that origin.
    #[must_use]
    pub fn api_key(mut self, key: ApiKey) -> Self {
        self.credentials = Some(key);
        self
    }

    /// Allow the API key to be presented to token endpoints chosen by discovery.
    ///
    /// Leave this unset when the well-known document is not a trusted source of
    /// credential destinations.
    #[must_use]
    pub fn allow_discovered_credentials(mut self, allow: bool) -> Self {
        self.allow_discovered_credentials = allow;
        self
    }

    /// Fetch the well-known document from this URL instead of `https://{tei-domain}/.well-known/tea`.
    ///
    /// The override is still subject to [`DestinationPolicy`]. It exists so tests
    /// and explicitly configured fixtures can supply the document without changing
    /// production URL construction.
    #[must_use]
    pub fn well_known_fetch_url(mut self, url: Url) -> Self {
        self.well_known_override = Some(url);
        self
    }

    /// Add a PEM trust anchor. System or webpki roots remain in use as well.
    #[must_use]
    pub fn add_root_pem(mut self, pem: impl Into<Vec<u8>>) -> Self {
        self.roots_pem.extend(pem.into());
        self.roots_pem.extend(b"\n");
        self
    }

    /// Build the client.
    ///
    /// # Errors
    ///
    /// Returns an error when TLS or the HTTP client cannot be constructed.
    pub fn build(self) -> Result<Client, ClientError> {
        if self.limits.max_in_flight == 0 {
            return Err(ClientError::Model(cdx_tea_core::ModelError::Invalid(
                "max_in_flight must be at least 1".into(),
            )));
        }
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(self.limits.connect_timeout)
            .timeout(self.limits.request_timeout)
            .user_agent(concat!("tea-sdk/", env!("CARGO_PKG_VERSION")))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .https_only(false)
            .dns_resolver(Arc::new(PolicyResolver {
                policy: self.policy.clone(),
            }));
        if !self.roots_pem.is_empty() {
            let cert = reqwest::Certificate::from_pem(&self.roots_pem)
                .map_err(|err| ClientError::Transport(format!("invalid trust anchor: {err}")))?;
            builder = builder.add_root_certificate(cert);
        }
        let http = builder
            .build()
            .map_err(|err| ClientError::Transport(format!("HTTP client setup failed: {err}")))?;
        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                semaphore: Arc::new(Semaphore::new(self.limits.max_in_flight)),
                limits: self.limits,
                policy: self.policy,
                supported: self.supported,
                credentials: self.credentials,
                credential_origins: Mutex::new(BTreeSet::new()),
                allow_discovered_credentials: self.allow_discovered_credentials,
                tokens: Mutex::new(std::collections::HashMap::new()),
                well_known_override: self.well_known_override,
            }),
        })
    }
}

impl Client {
    /// Production client for TEA 1.0.0.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP client cannot be constructed.
    pub fn new() -> Result<Self, ClientError> {
        ClientBuilder::new().build()
    }

    /// Start a builder.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Bind a caller-supplied API base.
    ///
    /// The base origin becomes an allowed destination for this client's API key.
    ///
    /// # Errors
    ///
    /// Returns an error when the base URL or version is not usable.
    pub fn service(&self, root: &str, version: &str) -> Result<crate::Service, ClientError> {
        let root_url = parse_api_base(root)?;
        self.inner.policy.check_url(&root_url)?;
        let version = ApiVersion::parse(version)?;
        self.authorize_origin(&root_url)?;
        crate::Service::from_parts(self.clone(), root_url, version.as_str().to_owned())
    }

    pub(crate) fn authorize_origin(&self, url: &Url) -> Result<(), ClientError> {
        let origin = origin_key(url)?;
        self.inner
            .credential_origins
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(origin);
        Ok(())
    }
}

tokio::task_local! {
    static REQUEST_URL: Url;
}

struct PolicyResolver {
    policy: DestinationPolicy,
}

impl reqwest::dns::Resolve for PolicyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        let policy = self.policy.clone();
        Box::pin(async move {
            let looked_up = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
            let mut allowed = Vec::new();
            for addr in looked_up {
                let ip = addr.ip();
                if address_allowed(&policy, ip) {
                    allowed.push(SocketAddr::new(ip, addr.port()));
                }
            }
            if allowed.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "resolved addresses are not allowed by destination policy",
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let addrs: reqwest::dns::Addrs = Box::new(allowed.into_iter());
            Ok(addrs)
        })
    }
}

/// A non-public address is kept only for the request URL's exact origin.
///
/// When the resolver runs outside a request scope, only public addresses are kept.
fn address_allowed(policy: &DestinationPolicy, ip: std::net::IpAddr) -> bool {
    REQUEST_URL
        .try_with(|url| policy.allows_resolved_ip(url, ip).is_ok())
        .unwrap_or_else(|_| classify_ip(ip).is_public())
}

pub(crate) struct RawResponse {
    pub status: StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: Bytes,
    pub final_url: Url,
    /// Artifact bodies are decoded while they are read. JSON bodies are not.
    pub content_decoded: bool,
}

pub(crate) struct Call {
    pub method: Method,
    pub url: Url,
    pub api_prefix: Option<Url>,
    pub artifact: bool,
    pub body: Option<String>,
    pub content_type: Option<&'static str>,
    pub basic_auth: bool,
    pub if_none_match: Option<String>,
    pub max_body: usize,
}

impl ClientInner {
    pub(crate) async fn send(&self, call: Call) -> Result<RawResponse, ClientError> {
        let mut attempt = 0u32;
        let mut refreshed = false;
        loop {
            match self.send_once(&call).await {
                Ok(response)
                    if response.status == StatusCode::UNAUTHORIZED
                        && !call.basic_auth
                        && !refreshed =>
                {
                    let challenge =
                        header_str(&response.headers, WWW_AUTHENTICATE).map(str::to_owned);
                    if self.should_refresh_token(challenge.as_deref(), call.api_prefix.as_ref()) {
                        let prefix = call
                            .api_prefix
                            .as_ref()
                            .ok_or(ClientError::AuthenticationRequired)?;
                        self.fetch_token(prefix).await?;
                        refreshed = true;
                        continue;
                    }
                    if challenge.as_deref().is_some_and(is_bearer_challenge) {
                        return Err(ClientError::AuthenticationRequired);
                    }
                    return Ok(response);
                }
                Ok(response) => return Ok(response),
                Err(ClientError::Timeout) | Err(ClientError::Transport(_))
                    if call.method == Method::GET && attempt < self.limits.max_retries =>
                {
                    attempt += 1;
                    self.backoff(attempt).await;
                }
                Err(other) => return Err(other),
            }
        }
    }

    async fn send_once(&self, call: &Call) -> Result<RawResponse, ClientError> {
        let mut url = call.url.clone();
        let mut redirects = 0usize;
        loop {
            self.policy.check_url(&url)?;
            let _permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| ClientError::Transport("request limiter closed".into()))?;
            let mut request = self.http.request(call.method.clone(), url.clone());
            if call.artifact {
                request = request.header("accept-encoding", "gzip, deflate, identity");
            }
            if let Some(etag) = &call.if_none_match {
                request = request.header("if-none-match", etag);
            }
            if call.basic_auth {
                let header = self.basic_header(&url)?;
                request = request.header(AUTHORIZATION, header);
            } else if let Some(token) = self.bearer_for(&url, call.api_prefix.as_ref()) {
                request = request.header(AUTHORIZATION, format!("Bearer {token}"));
            }
            if let Some(content_type) = call.content_type {
                request = request.header(CONTENT_TYPE, content_type);
            }
            if let Some(body) = &call.body {
                request = request.body(body.clone());
            }
            tracing::debug!(%url, method = %call.method, "tea request");
            let response = REQUEST_URL
                .scope(url.clone(), request.send())
                .await
                .map_err(ClientError::transport)?;
            let status = response.status();
            if matches!(
                status,
                StatusCode::MOVED_PERMANENTLY
                    | StatusCode::FOUND
                    | StatusCode::SEE_OTHER
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::PERMANENT_REDIRECT
            ) && call.method != Method::POST
            {
                redirects += 1;
                if redirects > self.limits.max_redirects {
                    return Err(ClientError::TooManyRedirects);
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(ClientError::BadRedirect)?;
                let next = url.join(location).map_err(|_| ClientError::BadRedirect)?;
                self.policy.check_url(&next)?;
                drop(response);
                url = next;
                continue;
            }
            if status == StatusCode::SERVICE_UNAVAILABLE && call.method == Method::GET {
                drop(response);
                return Err(ClientError::Transport("HTTP 503".into()));
            }
            let headers = response.headers().clone();
            let metadata_only = call.method == Method::HEAD || status == StatusCode::NOT_MODIFIED;
            let (body, content_decoded) = if metadata_only {
                // Content-Length on HEAD and 304 describes the selected
                // representation, not a message body.
                drop(response);
                (Bytes::new(), false)
            } else if call.artifact {
                let coding = header_str(&headers, CONTENT_ENCODING).map(str::to_owned);
                let decoded = read_decoded(response, call.max_body, coding.as_deref()).await?;
                (Bytes::from(decoded), true)
            } else {
                (read_limited(response, call.max_body).await?, false)
            };
            tracing::debug!(%url, status = status.as_u16(), "tea response");
            return Ok(RawResponse {
                status,
                headers,
                body,
                final_url: url,
                content_decoded,
            });
        }
    }

    fn should_refresh_token(&self, challenge: Option<&str>, api_prefix: Option<&Url>) -> bool {
        let Some(challenge) = challenge else {
            return false;
        };
        if !is_bearer_challenge(challenge) {
            return false;
        }
        let error = challenge_error(challenge);
        if matches!(error.as_deref(), Some(code) if !code.eq_ignore_ascii_case("invalid_token")) {
            return false;
        }
        let Some(prefix) = api_prefix else {
            return false;
        };
        self.credentials.is_some() && origin_allowed(self, prefix)
    }

    fn bearer_for(&self, url: &Url, api_prefix: Option<&Url>) -> Option<String> {
        let prefix = api_prefix?;
        if !url_under_prefix(url, prefix) {
            return None;
        }
        let origin = origin_key(url).ok()?;
        let tokens = self.tokens.lock().unwrap_or_else(|err| err.into_inner());
        let stored = tokens.get(&origin)?;
        if stored.expires_at.is_some_and(|when| when <= Instant::now()) {
            return None;
        }
        Some(stored.value.clone())
    }

    fn basic_header(&self, url: &Url) -> Result<String, ClientError> {
        if !origin_allowed(self, url) {
            return Err(ClientError::AuthenticationRequired);
        }
        let Some(key) = &self.credentials else {
            return Err(ClientError::AuthenticationRequired);
        };
        let id = form_encode(&key.id);
        let secret = form_encode(&key.secret);
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{id}:{secret}"),
        );
        Ok(format!("Basic {encoded}"))
    }

    pub(crate) async fn fetch_token(&self, api_prefix: &Url) -> Result<(), ClientError> {
        if !origin_allowed(self, api_prefix) {
            return Err(ClientError::AuthenticationRequired);
        }
        let url = api_prefix.join("token").map_err(|_| {
            ClientError::Model(cdx_tea_core::ModelError::Invalid("token URL".into()))
        })?;
        let call = Call {
            method: Method::POST,
            url,
            api_prefix: Some(api_prefix.clone()),
            artifact: false,
            body: Some("grant_type=client_credentials".into()),
            content_type: Some("application/x-www-form-urlencoded"),
            basic_auth: true,
            if_none_match: None,
            max_body: self.limits.max_json_bytes,
        };
        let response = self.send_once(&call).await?;
        if response.status == StatusCode::NOT_FOUND {
            return Err(ClientError::Api {
                status: 404,
                tea_error: tea_error_code(
                    header_str(&response.headers, CONTENT_TYPE),
                    &response.body,
                ),
                message: None,
            });
        }
        if !response.status.is_success() {
            let code = serde_json::from_slice::<serde_json::Value>(&response.body)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(|error| error.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| response.status.as_str().to_owned());
            return Err(ClientError::Token { error: code });
        }
        let token: TokenResponse = serde_json::from_slice(&response.body)?;
        let token = token.bearer()?;
        let expires_at = token.expires_in.and_then(|seconds| {
            Instant::now().checked_add(Duration::from_secs(seconds.saturating_sub(5)))
        });
        let origin = origin_key(api_prefix)?;
        self.tokens
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(
                origin,
                StoredToken {
                    value: token.access_token,
                    expires_at,
                },
            );
        Ok(())
    }

    pub(crate) async fn backoff(&self, attempt: u32) {
        let shift = attempt.min(5);
        let delay = self.limits.initial_backoff.saturating_mul(1 << shift);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

fn origin_allowed(inner: &ClientInner, url: &Url) -> bool {
    origin_key(url)
        .map(|origin| {
            inner
                .credential_origins
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .contains(&origin)
        })
        .unwrap_or(false)
}

fn is_bearer_challenge(value: &str) -> bool {
    value
        .split_whitespace()
        .next()
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("bearer"))
}

fn challenge_error(value: &str) -> Option<String> {
    value.split(',').find_map(|part| {
        let (name, raw) = part.trim().split_once('=')?;
        if name.trim().eq_ignore_ascii_case("error") {
            Some(raw.trim().trim_matches('"').to_owned())
        } else {
            None
        }
    })
}

pub(crate) fn url_under_prefix(url: &Url, prefix: &Url) -> bool {
    url.scheme() == prefix.scheme()
        && url.host_str() == prefix.host_str()
        && url.port_or_known_default() == prefix.port_or_known_default()
        && url.path().starts_with(prefix.path())
}

fn form_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

async fn read_decoded(
    response: reqwest::Response,
    max: usize,
    coding: Option<&str>,
) -> Result<Vec<u8>, ClientError> {
    if response
        .content_length()
        .is_some_and(|len| len > max as u64)
    {
        return Err(ClientError::TooLarge);
    }
    let mut decoder = ContentDecoder::new(coding, max)?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(ClientError::transport)?;
        decoder.push(&chunk)?;
    }
    Ok(decoder.finish()?)
}

async fn read_limited(response: reqwest::Response, max: usize) -> Result<Bytes, ClientError> {
    if response
        .content_length()
        .is_some_and(|len| len > max as u64)
    {
        return Err(ClientError::TooLarge);
    }
    let mut collected = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(ClientError::transport)?;
        if collected.len().saturating_add(chunk.len()) > max {
            return Err(ClientError::TooLarge);
        }
        collected.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(collected))
}

pub(crate) fn header_str(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

pub(crate) fn map_api_error(response: &RawResponse) -> ClientError {
    if response.status == StatusCode::FORBIDDEN {
        return ClientError::Forbidden;
    }
    let content_type = header_str(&response.headers, CONTENT_TYPE);
    let tea = tea_error_code(content_type, &response.body);
    let message = if tea.is_some() {
        serde_json::from_slice::<TeaErrorBody>(&response.body)
            .ok()
            .and_then(|body| body.message)
    } else {
        None
    };
    ClientError::Api {
        status: response.status.as_u16(),
        tea_error: tea,
        message,
    }
}

pub(crate) fn operation_url(
    prefix: &Url,
    path: &str,
    query: &[(&str, &str)],
) -> Result<Url, ClientError> {
    let mut url = prefix.join(path.trim_start_matches('/')).map_err(|_| {
        ClientError::Model(cdx_tea_core::ModelError::Invalid("operation URL".into()))
    })?;
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();
        for (name, value) in query {
            pairs.append_pair(name, value);
        }
    }
    Ok(url)
}

pub(crate) use reqwest::header::CONTENT_TYPE as CONTENT_TYPE_HEADER;
