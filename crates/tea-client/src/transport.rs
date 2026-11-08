//! HTTP transport with manual redirects, body limits, and credential scoping.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use libtea_core::checksum::ContentDecoder;
use libtea_core::http_cache::{
    cache_key, parse_retry_after, storage_decision, CacheControl, CacheEntryMeta, NotStored,
    STORED_HEADERS,
};
use libtea_core::{
    classify_ip, origin_key, parse_api_base, tea_error_code, ApiVersion, ChecksumPolicy,
    DestinationPolicy, SupportedApi, TeaErrorBody, TokenResponse,
};
use reqwest::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, LOCATION, RETRY_AFTER, VARY,
    WWW_AUTHENTICATE,
};
use reqwest::{Method, StatusCode};
use tokio::sync::Semaphore;
use url::Url;

use crate::cache::{now_secs, Cache, CacheConfig};
use crate::error::ClientError;

/// Bounds for untrusted TEA responses.
///
/// `#[non_exhaustive]`: start from [`Limits::default`] and use the `with_*`
/// setters (or assign fields on a default value).
#[derive(Debug, Clone)]
#[non_exhaustive]
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
    /// Extra attempts for idempotent GET transport failures, HTTP 429 and HTTP 503.
    pub max_retries: u32,
    /// Longest `Retry-After` the client will wait out before retrying a `429`
    /// or `503`. A longer one fails at once with [`ClientError::Throttled`],
    /// which carries the server's value.
    pub max_retry_after: Duration,
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
            max_retry_after: Duration::from_secs(30),
            max_discovery_attempts: 4,
            initial_backoff: Duration::from_millis(200),
            max_in_flight: 8,
        }
    }
}

impl Limits {
    /// Set [`Self::max_discovery_bytes`].
    #[must_use]
    pub fn with_max_discovery_bytes(mut self, value: usize) -> Self {
        self.max_discovery_bytes = value;
        self
    }

    /// Set [`Self::max_json_bytes`].
    #[must_use]
    pub fn with_max_json_bytes(mut self, value: usize) -> Self {
        self.max_json_bytes = value;
        self
    }

    /// Set [`Self::max_artifact_bytes`].
    #[must_use]
    pub fn with_max_artifact_bytes(mut self, value: usize) -> Self {
        self.max_artifact_bytes = value;
        self
    }

    /// Set [`Self::max_redirects`].
    #[must_use]
    pub fn with_max_redirects(mut self, value: usize) -> Self {
        self.max_redirects = value;
        self
    }

    /// Set [`Self::request_timeout`].
    #[must_use]
    pub fn with_request_timeout(mut self, value: Duration) -> Self {
        self.request_timeout = value;
        self
    }

    /// Set [`Self::connect_timeout`].
    #[must_use]
    pub fn with_connect_timeout(mut self, value: Duration) -> Self {
        self.connect_timeout = value;
        self
    }

    /// Set [`Self::max_retries`].
    #[must_use]
    pub fn with_max_retries(mut self, value: u32) -> Self {
        self.max_retries = value;
        self
    }

    /// Set [`Self::max_discovery_attempts`].
    #[must_use]
    pub fn with_max_discovery_attempts(mut self, value: u32) -> Self {
        self.max_discovery_attempts = value;
        self
    }

    /// Set [`Self::initial_backoff`].
    #[must_use]
    pub fn with_initial_backoff(mut self, value: Duration) -> Self {
        self.initial_backoff = value;
        self
    }

    /// Set [`Self::max_retry_after`].
    #[must_use]
    pub fn with_max_retry_after(mut self, value: Duration) -> Self {
        self.max_retry_after = value;
        self
    }

    /// Set [`Self::max_in_flight`].
    #[must_use]
    pub fn with_max_in_flight(mut self, value: usize) -> Self {
        self.max_in_flight = value;
        self
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

/// Single-flight state for one origin's token exchange.
#[derive(Default)]
struct RefreshSlot {
    /// Number of exchanges for this origin that ran to completion.
    completed: AtomicU64,
    /// Failure of the most recent completed exchange, `None` after a success.
    last_failure: tokio::sync::Mutex<Option<SharedFailure>>,
}

/// A caller's hold on one origin's [`RefreshSlot`].
///
/// Dropping the last lease removes the slot from the map, including when the
/// holder is cancelled. The map therefore only holds origins with a refresh in
/// progress or queued, and a stored failure never outlives its burst.
struct SlotLease<'a> {
    inner: &'a ClientInner,
    origin: String,
    slot: Arc<RefreshSlot>,
}

impl<'a> SlotLease<'a> {
    fn acquire(inner: &'a ClientInner, origin: String) -> Self {
        let mut slots = inner
            .token_refresh
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let slot = Arc::clone(slots.entry(origin.clone()).or_default());
        Self {
            inner,
            origin,
            slot,
        }
    }
}

impl Drop for SlotLease<'_> {
    fn drop(&mut self) {
        let mut slots = self
            .inner
            .token_refresh
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        // Leases are only cloned under this lock, so the count is exact here:
        // one reference in the map and one held by this lease.
        let last = slots
            .get(&self.origin)
            .is_some_and(|stored| Arc::ptr_eq(stored, &self.slot))
            && Arc::strong_count(&self.slot) == 2;
        if last {
            slots.remove(&self.origin);
        }
    }
}

/// Cloneable summary of a token exchange failure, handed to queued waiters.
#[derive(Clone)]
enum SharedFailure {
    Timeout,
    Token(String),
    Api {
        status: u16,
        tea_error: Option<String>,
    },
    AuthenticationRequired,
    Other,
}

impl SharedFailure {
    fn from_error(error: &ClientError) -> Self {
        match error {
            ClientError::Timeout => Self::Timeout,
            ClientError::Token { error } => Self::Token(error.clone()),
            ClientError::Api {
                status, tea_error, ..
            } => Self::Api {
                status: *status,
                tea_error: tea_error.clone(),
            },
            ClientError::AuthenticationRequired => Self::AuthenticationRequired,
            _ => Self::Other,
        }
    }

    fn to_error(&self) -> ClientError {
        match self {
            Self::Timeout => ClientError::Timeout,
            Self::Token(error) => ClientError::Token {
                error: error.clone(),
            },
            Self::Api { status, tea_error } => ClientError::Api {
                status: *status,
                tea_error: tea_error.clone(),
                message: None,
            },
            Self::AuthenticationRequired => ClientError::AuthenticationRequired,
            Self::Other => ClientError::Transport("token exchange failed".into()),
        }
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
    pub(crate) checksum_policy: ChecksumPolicy,
    tokens: Mutex<std::collections::HashMap<String, StoredToken>>,
    /// Per-origin single-flight gates for `POST /token`.
    ///
    /// At most one `POST /token` per origin runs at a time on a given
    /// [`Client`]; different origins do not wait for each other. Waiters
    /// re-check the bearer cache under the lock and skip another exchange when
    /// a usable bearer is already stored. Waiters that queued while an
    /// exchange for that origin was in flight share its failure instead of
    /// repeating it. See `docs/auth.md` for the exact bound. An entry exists
    /// only while a refresh for that origin is running or queued
    /// ([`SlotLease`] removes it), so origin churn cannot grow the map.
    /// Ordinary GETs are not held behind these locks.
    token_refresh: Mutex<std::collections::HashMap<String, Arc<RefreshSlot>>>,
    semaphore: Arc<Semaphore>,
    pub(crate) well_known_override: Option<Url>,
    pub(crate) cache: Option<Arc<Cache>>,
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
    checksum_policy: ChecksumPolicy,
    ignore_env_proxies: bool,
    lenient_versions: bool,
    cache: Option<CacheConfig>,
    offline: bool,
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
            checksum_policy: ChecksumPolicy::default(),
            ignore_env_proxies: false,
            lenient_versions: false,
            cache: None,
            offline: false,
        }
    }
}

impl ClientBuilder {
    /// Create a builder with production defaults: HTTPS, public addresses, TEA 1.0.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep a persistent response cache in `dir` (*experimental*; off by default).
    ///
    /// `GET` responses are stored and revalidated with `ETag` / `Last-Modified`
    /// conditional requests. The directory is created private (mode 0700, files
    /// 0600 on Unix); a symbolic link or a directory open to other users makes
    /// [`ClientBuilder::build`] fail. Responses to requests that carried
    /// credentials, `Cache-Control: no-store` and (by default) `private`
    /// responses are not stored. Cache contents are never trusted: artifact
    /// bytes are checked against their declared checksums on every use.
    /// See `docs/cache.md` and [`CacheConfig`] for the limits.
    #[must_use]
    pub fn cache_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.cache = Some(CacheConfig::new(dir));
        self
    }

    /// Enable the cache with explicit settings (*experimental*).
    #[must_use]
    pub fn cache(mut self, config: CacheConfig) -> Self {
        self.cache = Some(config);
        self
    }

    /// Answer only from the cache and never open a connection (*experimental*).
    ///
    /// A cacheable `GET` that is not in the cache (or whose entry is older than
    /// [`CacheConfig::max_age`]) fails with [`ClientError::Offline`]; any other
    /// request fails the same way. Requires [`ClientBuilder::cache_dir`] or
    /// [`ClientBuilder::cache`]; [`ClientBuilder::build`] fails without one.
    #[must_use]
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
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

    /// Accept API versions that are not exactly a supported one (default `false`).
    ///
    /// When `true`, `/.well-known/tea` and `/discovery` may advertise `1` or
    /// `1.0` (read as `1.0.0`; the advertised spelling stays in the request
    /// path, so `1` calls `/v1/…`), and any *stable* version with the same
    /// major as a supported version is accepted, for example `1.0.1` and
    /// `1.1.0` for a client that supports `1.0.0`. Pre-releases (`1.2.0-rc.1`)
    /// are accepted only on an exact match.
    ///
    /// Selection order: exact supported versions first (highest version, then
    /// priority), then same-major stable versions (highest first). An exact
    /// match is never outranked by a version this client was not written for.
    ///
    /// **Risk:** this client's models were checked against the pinned 1.0.0
    /// schemas only. A server that passes the version check may still send
    /// responses that fail to parse or that are misread. The default (`false`)
    /// is unchanged from earlier behavior.
    #[must_use]
    pub fn lenient_versions(mut self, lenient: bool) -> Self {
        self.lenient_versions = lenient;
        self
    }

    /// Accept artifact downloads whose only declared checksums use weak
    /// algorithms (SHA-1).
    ///
    /// Off by default: such a download fails with [`ClientError::Integrity`]
    /// and [`libtea_core::IntegrityOutcome::Weak`]. When set, a full match is
    /// reported as [`libtea_core::IntegrityOutcome::Verified`] and each
    /// [`libtea_core::ChecksumCheck::weak`] stays `true`.
    #[must_use]
    pub fn allow_weak_checksums(mut self, allow: bool) -> Self {
        self.checksum_policy = self.checksum_policy.with_allow_weak(allow);
        self
    }

    /// Ignore `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` (and lowercase forms).
    ///
    /// By default environment proxies are honored. Through a proxy, the
    /// proxy resolves the target hostname, so the DNS-based
    /// [`DestinationPolicy`] checks are **not** enforced for hostnames; only
    /// scheme, userinfo, literal-IP, and redirect checks still apply. Set this
    /// to `true` to connect directly and keep the DNS checks.
    #[must_use]
    pub fn ignore_env_proxies(mut self, ignore: bool) -> Self {
        self.ignore_env_proxies = ignore;
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
            return Err(ClientError::Model(libtea_core::ModelError::Invalid(
                "max_in_flight must be at least 1".into(),
            )));
        }
        let cache = match self.cache {
            Some(config) => Some(Arc::new(Cache::open(config, self.offline)?)),
            None if self.offline => {
                return Err(ClientError::Model(libtea_core::ModelError::Invalid(
                    "offline mode needs a cache directory".into(),
                )));
            }
            None => None,
        };
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(self.limits.connect_timeout)
            .timeout(self.limits.request_timeout)
            .user_agent(concat!("libtea/", env!("CARGO_PKG_VERSION")))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .https_only(false)
            .dns_resolver(Arc::new(PolicyResolver {
                policy: self.policy.clone(),
            }));
        if self.ignore_env_proxies {
            builder = builder.no_proxy();
        }
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
                supported: if self.lenient_versions {
                    self.supported.with_lenient(true)
                } else {
                    self.supported
                },
                credentials: self.credentials,
                credential_origins: Mutex::new(BTreeSet::new()),
                allow_discovered_credentials: self.allow_discovered_credentials,
                tokens: Mutex::new(std::collections::HashMap::new()),
                token_refresh: Mutex::new(std::collections::HashMap::new()),
                checksum_policy: self.checksum_policy,
                well_known_override: self.well_known_override,
                cache,
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
        let version = if self.inner.supported.is_lenient() {
            ApiVersion::parse_lenient(version)?
        } else {
            ApiVersion::parse(version)?
        };
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
    /// Set when the response may be stored once its body has been checked.
    pending_cache: Option<Box<PendingCache>>,
    /// Set when the body came out of the cache, so a failed check can evict it.
    from_cache: Option<Box<CacheOrigin>>,
    /// Bearer token attached to the request that produced this response.
    presented_bearer: Option<String>,
}

/// Everything needed to store a response after the caller has checked it.
pub(crate) struct PendingCache {
    cache: Arc<Cache>,
    key: String,
    request_url: String,
    headers: Vec<(String, String)>,
    /// A revalidated entry whose age restarts when the body has been checked.
    /// `None` for a response that is new to the cache.
    meta: Option<CacheEntryMeta>,
}

/// Where a cached body came from.
pub(crate) struct CacheOrigin {
    cache: Arc<Cache>,
    key: String,
}

impl RawResponse {
    /// Store this response in the cache. Call only after its body passed the
    /// checks that matter (artifact checksums); a no-op when nothing is pending.
    pub(crate) async fn commit_cache(&self) {
        let Some(pending) = &self.pending_cache else {
            return;
        };
        if self.body.len() as u64 > pending.cache.config.max_entry_bytes {
            return;
        }
        let meta = match &pending.meta {
            Some(meta) => meta.clone(),
            None => CacheEntryMeta::new(
                &pending.key,
                &pending.request_url,
                200,
                now_secs(),
                self.content_decoded,
                &pending.headers,
            ),
        };
        pending
            .cache
            .store(&pending.key, meta, self.body.to_vec())
            .await;
    }

    /// The caller's check of this body failed. If it came from the cache, drop
    /// the entry so it is neither served again nor renewed by a `304`.
    pub(crate) async fn evict_cached(&self) {
        if let Some(origin) = &self.from_cache {
            origin.cache.remove(&origin.key).await;
        }
    }
}

/// A response whose headers are read and whose body is not.
struct Opened {
    status: StatusCode,
    headers: reqwest::header::HeaderMap,
    final_url: Url,
    presented_bearer: Option<String>,
    response: reqwest::Response,
}

/// First stage of a streamed artifact download.
pub(crate) enum StreamStart {
    /// A success status; the body is unread.
    Body(OpenBody),
    /// `304`, or an error status with its bounded body, for `map_api_error`.
    Other(RawResponse),
}

/// A success response with an unread body.
pub(crate) struct OpenBody {
    pub status: StatusCode,
    /// Whether the request carried a bearer token.
    pub authenticated: bool,
    pub headers: reqwest::header::HeaderMap,
    pub final_url: Url,
    response: reqwest::Response,
}

impl OpenBody {
    /// The next raw (still content-coded) body chunk.
    pub(crate) async fn chunk(&mut self) -> Result<Option<Bytes>, ClientError> {
        self.response.chunk().await.map_err(ClientError::transport)
    }
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
    pub if_modified_since: Option<String>,
    pub max_body: usize,
}

impl ClientInner {
    /// Send a request, through the cache when one is configured and the call is
    /// a plain `GET`.
    pub(crate) async fn send(&self, mut call: Call) -> Result<RawResponse, ClientError> {
        let Some(cache) = self.cache.clone() else {
            return self.send_network(call).await;
        };
        let eligible = call.method == Method::GET
            && call.if_none_match.is_none()
            && call.if_modified_since.is_none();
        let key = if eligible {
            self.cache_key_for(&call.url)
        } else {
            None
        };
        let Some(key) = key else {
            if cache.offline {
                return Err(ClientError::Offline);
            }
            return self.send_network(call).await;
        };

        // A cache hit is subject to the same destination rules as a request:
        // another client, with a more permissive policy, may have stored it.
        self.policy.check_url(&call.url)?;
        let mut existing = cache.load(&key).await;
        // ...and to the same size limit, as it is today. Limits can be lowered
        // after an entry was stored, and an entry can be planted.
        if existing
            .as_ref()
            .is_some_and(|(_, body)| body.len() > call.max_body)
        {
            cache.remove(&key).await;
            if cache.offline {
                return Err(ClientError::TooLarge);
            }
            existing = None;
        }
        if cache.offline {
            return existing
                .map(|(meta, body)| {
                    let mut raw = raw_from_entry(&call.url, &meta, body);
                    raw.from_cache = Some(Box::new(CacheOrigin {
                        cache: Arc::clone(&cache),
                        key: key.clone(),
                    }));
                    raw
                })
                .ok_or(ClientError::Offline);
        }
        if let Some((meta, _)) = &existing {
            call.if_none_match = meta.header("etag").map(str::to_owned);
            call.if_modified_since = meta.header("last-modified").map(str::to_owned);
        }
        let url = call.url.clone();
        let artifact = call.artifact;
        let mut response = self.send_network(call).await?;

        match response.status.as_u16() {
            304 => {
                let Some((meta, body)) = existing else {
                    return Ok(response);
                };
                let revalidated = self.revalidated_meta(
                    &cache,
                    meta.clone(),
                    &response.headers,
                    response.presented_bearer.is_some(),
                );
                let mut raw = raw_from_entry(&url, revalidated.as_ref().unwrap_or(&meta), body);
                raw.from_cache = Some(Box::new(CacheOrigin {
                    cache: Arc::clone(&cache),
                    key: key.clone(),
                }));
                match revalidated {
                    // The entry's age restarts only after the body has passed the
                    // caller's checks; a JSON body has none, so renew it now.
                    Some(meta) => {
                        raw.pending_cache = Some(Box::new(PendingCache {
                            cache: Arc::clone(&cache),
                            key,
                            request_url: display_url(&url),
                            headers: Vec::new(),
                            meta: Some(meta),
                        }));
                        if !artifact {
                            raw.commit_cache().await;
                        }
                    }
                    // The new directives forbid keeping it.
                    None => cache.remove(&key).await,
                }
                Ok(raw)
            }
            200 => {
                let headers = storable_headers(&response.headers);
                let decision = decide_storage(
                    &response.headers,
                    response.presented_bearer.is_some(),
                    cache.config.authenticated,
                );
                match decision {
                    Ok(()) => {
                        let pending = PendingCache {
                            cache: Arc::clone(&cache),
                            key,
                            request_url: display_url(&url),
                            headers,
                            meta: None,
                        };
                        response.pending_cache = Some(Box::new(pending));
                        // Artifact bytes are stored only after the caller has
                        // checked them; everything else is stored now.
                        if !artifact {
                            response.commit_cache().await;
                        }
                    }
                    Err(NotStored::NoStore) => cache.remove(&key).await,
                    Err(_) => {}
                }
                Ok(response)
            }
            404 | 410 => {
                cache.remove(&key).await;
                Ok(response)
            }
            _ => Ok(response),
        }
    }

    /// Cache key for a `GET` of `url`, or `None` when the origin has no key.
    pub(crate) fn cache_key_for(&self, url: &Url) -> Option<String> {
        let mut normalized = url.clone();
        normalized.set_fragment(None);
        let _ = normalized.set_username("");
        let _ = normalized.set_password(None);
        let origin = origin_key(&normalized).ok()?;
        Some(cache_key(
            "GET",
            normalized.as_str(),
            &origin,
            self.credentials.as_ref().map(|key| key.id.as_str()),
        ))
    }

    /// A `304` confirmed the stored entry: take the new validators and
    /// directives and restart its age. Returns the entry as it should now be
    /// stored, or `None` when the new directives forbid keeping it. Nothing is
    /// written here: the caller stores it once the body has been checked.
    pub(crate) fn revalidated_meta(
        &self,
        cache: &Cache,
        mut meta: CacheEntryMeta,
        headers: &reqwest::header::HeaderMap,
        authenticated: bool,
    ) -> Option<CacheEntryMeta> {
        merge_validated_headers(&mut meta, headers);
        meta.validated_at = now_secs().max(meta.stored_at);
        let lines: Vec<&str> = meta.header("cache-control").into_iter().collect();
        storage_decision(
            200,
            &CacheControl::parse(&lines),
            meta.header("vary"),
            authenticated,
            cache.config.authenticated,
        )
        .is_ok()
        .then_some(meta)
    }

    async fn wait_after_throttle(
        &self,
        attempt: u32,
        error: ClientError,
    ) -> Result<(), ClientError> {
        let seconds = match &error {
            ClientError::Throttled {
                retry_after_secs, ..
            } => *retry_after_secs,
            _ => return Err(error),
        };
        match seconds {
            Some(seconds) => {
                let wait = Duration::from_secs(seconds);
                if wait > self.limits.max_retry_after {
                    return Err(error);
                }
                if !wait.is_zero() {
                    tokio::time::sleep(wait).await;
                }
            }
            None => self.backoff(attempt).await,
        }
        Ok(())
    }

    async fn send_network(&self, call: Call) -> Result<RawResponse, ClientError> {
        let mut attempt = 0u32;
        let mut refreshed = false;
        loop {
            match self.send_once(&call).await {
                Ok(response)
                    if response.status == StatusCode::UNAUTHORIZED
                        && self
                            .retry_after_unauthorized(
                                &call,
                                &response.headers,
                                response.presented_bearer.as_deref(),
                                &mut refreshed,
                            )
                            .await? =>
                {
                    continue;
                }
                Ok(response) => return Ok(response),
                Err(ClientError::Timeout) | Err(ClientError::Transport(_))
                    if call.method == Method::GET && attempt < self.limits.max_retries =>
                {
                    attempt += 1;
                    self.backoff(attempt).await;
                }
                Err(error @ ClientError::Throttled { .. })
                    if call.method == Method::GET && attempt < self.limits.max_retries =>
                {
                    attempt += 1;
                    self.wait_after_throttle(attempt, error).await?;
                }
                Err(other) => return Err(other),
            }
        }
    }

    /// Decide what a `401` means for `call`. `Ok(true)` asks the caller to send
    /// the request again with a refreshed token; `Ok(false)` hands the `401`
    /// response back.
    async fn retry_after_unauthorized(
        &self,
        call: &Call,
        headers: &reqwest::header::HeaderMap,
        presented_bearer: Option<&str>,
        refreshed: &mut bool,
    ) -> Result<bool, ClientError> {
        if call.basic_auth {
            return Ok(false);
        }
        if *refreshed {
            // Drop the bearer token this retry presented, and only when
            // that value is still the cache entry. A newer token stored
            // for this origin while the 401 was in flight stays cached.
            if let Some(presented) = presented_bearer {
                self.discard_token_if_current(call.api_prefix.as_ref(), presented);
            }
            return Ok(false);
        }
        let challenge = header_str(headers, WWW_AUTHENTICATE).map(str::to_owned);
        if self.should_refresh_token(challenge.as_deref(), call.api_prefix.as_ref()) {
            let prefix = call
                .api_prefix
                .as_ref()
                .ok_or(ClientError::AuthenticationRequired)?;
            self.refresh_token_coalesced(prefix, presented_bearer)
                .await?;
            *refreshed = true;
            return Ok(true);
        }
        if challenge.as_deref().is_some_and(is_bearer_challenge) {
            return Err(ClientError::AuthenticationRequired);
        }
        Ok(false)
    }

    /// Like [`ClientInner::send`] for an artifact `GET`, but a success body is
    /// left unread so the caller can stream it.
    ///
    /// Redirects, destination policy, token refresh and the `401`/`503`
    /// handling are shared with `send`. Because a stream cannot be replayed, a
    /// transport failure while the body is read is not retried; failures
    /// before the body starts are retried like any other `GET`.
    pub(crate) async fn send_streaming(&self, call: Call) -> Result<StreamStart, ClientError> {
        let mut attempt = 0u32;
        let mut refreshed = false;
        loop {
            match self.open_once(&call).await {
                Ok(opened)
                    if opened.status == StatusCode::UNAUTHORIZED
                        && self
                            .retry_after_unauthorized(
                                &call,
                                &opened.headers,
                                opened.presented_bearer.as_deref(),
                                &mut refreshed,
                            )
                            .await? =>
                {
                    continue;
                }
                Ok(opened) => return self.start_stream(opened, &call).await,
                Err(ClientError::Timeout) | Err(ClientError::Transport(_))
                    if call.method == Method::GET && attempt < self.limits.max_retries =>
                {
                    attempt += 1;
                    self.backoff(attempt).await;
                }
                Err(error @ ClientError::Throttled { .. })
                    if call.method == Method::GET && attempt < self.limits.max_retries =>
                {
                    attempt += 1;
                    self.wait_after_throttle(attempt, error).await?;
                }
                Err(other) => return Err(other),
            }
        }
    }

    async fn start_stream(&self, opened: Opened, call: &Call) -> Result<StreamStart, ClientError> {
        if opened.status.is_success() {
            if opened
                .response
                .content_length()
                .is_some_and(|len| len > call.max_body as u64)
            {
                return Err(ClientError::TooLarge);
            }
            return Ok(StreamStart::Body(OpenBody {
                status: opened.status,
                authenticated: opened.presented_bearer.is_some(),
                headers: opened.headers,
                final_url: opened.final_url,
                response: opened.response,
            }));
        }
        // 304 and error statuses carry no artifact. Error bodies are read as
        // ordinary bounded JSON so the same error mapping applies.
        let metadata_only = opened.status == StatusCode::NOT_MODIFIED;
        let body = if metadata_only {
            Bytes::new()
        } else {
            read_limited(opened.response, self.limits.max_json_bytes).await?
        };
        Ok(StreamStart::Other(RawResponse {
            status: opened.status,
            headers: opened.headers,
            body,
            final_url: opened.final_url,
            content_decoded: false,
            pending_cache: None,
            from_cache: None,
            presented_bearer: opened.presented_bearer,
        }))
    }

    async fn send_once(&self, call: &Call) -> Result<RawResponse, ClientError> {
        let Opened {
            status,
            headers,
            final_url,
            presented_bearer,
            response,
        } = self.open_once(call).await?;
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
        tracing::debug!(
            url = %redact_url_for_log(&final_url),
            status = status.as_u16(),
            "tea response"
        );
        Ok(RawResponse {
            status,
            headers,
            body,
            final_url,
            content_decoded,
            pending_cache: None,
            from_cache: None,
            presented_bearer,
        })
    }

    /// Send the request, follow redirects under the destination policy and
    /// return the final response with its body unread.
    async fn open_once(&self, call: &Call) -> Result<Opened, ClientError> {
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
            if let Some(date) = &call.if_modified_since {
                request = request.header("if-modified-since", date);
            }
            let presented_bearer = if call.basic_auth {
                let header = self.basic_header(&url)?;
                request = request.header(AUTHORIZATION, header);
                None
            } else if let Some(token) = self.bearer_for(&url, call.api_prefix.as_ref()) {
                request = request.header(AUTHORIZATION, format!("Bearer {token}"));
                Some(token)
            } else {
                None
            };
            if let Some(content_type) = call.content_type {
                request = request.header(CONTENT_TYPE, content_type);
            }
            if let Some(body) = &call.body {
                request = request.body(body.clone());
            }
            tracing::debug!(
                url = %redact_url_for_log(&url),
                method = %call.method,
                "tea request"
            );
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
            if matches!(
                status,
                StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
            ) && call.method == Method::GET
            {
                let retry_after_secs = response
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| parse_retry_after(value, now_secs()));
                drop(response);
                return Err(ClientError::Throttled {
                    status: status.as_u16(),
                    retry_after_secs,
                });
            }
            let headers = response.headers().clone();
            return Ok(Opened {
                status,
                headers,
                final_url: url,
                presented_bearer,
                response,
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

    /// Coalesce concurrent token exchanges for one origin.
    ///
    /// Waiters that find a cached bearer different from the value that just
    /// received `401` reuse that token and skip another `POST /token`. A
    /// waiter that queued while an exchange was in flight and finds that it
    /// failed returns the same failure. If the exchanging task is cancelled,
    /// nothing is recorded and the next waiter performs its own exchange.
    async fn refresh_token_coalesced(
        &self,
        api_prefix: &Url,
        presented: Option<&str>,
    ) -> Result<(), ClientError> {
        let lease = SlotLease::acquire(self, origin_key(api_prefix)?);
        let slot = &lease.slot;
        let seen = slot.completed.load(Ordering::Acquire);
        let mut last_failure = slot.last_failure.lock().await;
        if let Some(current) = self.bearer_for(api_prefix, Some(api_prefix)) {
            let reusable = match presented {
                Some(old) => current != old,
                None => true,
            };
            if reusable {
                return Ok(());
            }
        }
        if slot.completed.load(Ordering::Acquire) != seen {
            if let Some(failure) = last_failure.as_ref() {
                return Err(failure.to_error());
            }
        }
        let result = self.fetch_token(api_prefix).await;
        *last_failure = result.as_ref().err().map(SharedFailure::from_error);
        slot.completed.fetch_add(1, Ordering::Release);
        result
    }

    pub(crate) async fn fetch_token(&self, api_prefix: &Url) -> Result<(), ClientError> {
        if !origin_allowed(self, api_prefix) {
            return Err(ClientError::AuthenticationRequired);
        }
        let url = api_prefix.join("token").map_err(|_| {
            ClientError::Model(libtea_core::ModelError::Invalid("token URL".into()))
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
            if_modified_since: None,
            max_body: self.limits.max_json_bytes,
        };
        let response = self.send_once(&call).await?;
        if response.status == StatusCode::NOT_FOUND {
            return Err(ClientError::Api {
                status: 404,
                tea_error: tea_error_code(
                    header_str(&response.headers, CONTENT_TYPE),
                    &response.body,
                )
                .map(|code| libtea_core::escape_control_chars(&code)),
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
            return Err(ClientError::Token {
                error: libtea_core::escape_control_chars(&code),
            });
        }
        let token: TokenResponse = serde_json::from_slice(&response.body)?;
        let token = token.bearer()?;
        let expires_at = token_expires_at(token.expires_in, Instant::now());
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

    fn discard_token_if_current(&self, api_prefix: Option<&Url>, presented: &str) {
        let Some(prefix) = api_prefix else {
            return;
        };
        let Ok(origin) = origin_key(prefix) else {
            return;
        };
        let mut tokens = self.tokens.lock().unwrap_or_else(|err| err.into_inner());
        let still_current = tokens
            .get(&origin)
            .is_some_and(|stored| stored.value == presented);
        if still_current {
            tokens.remove(&origin);
        }
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

/// Local expiry of a token response.
///
/// TEA leaves lifetime to the implementation and says servers should send
/// `expires_in`. A five-second margin applies only when the lifetime is longer
/// than that margin, so a short positive lifetime is still usable. `0` is
/// already expired. A missing value, or a lifetime `Instant` cannot represent,
/// has no local expiry and is replaced when the server returns `invalid_token`.
fn token_expires_at(expires_in: Option<u64>, now: Instant) -> Option<Instant> {
    let seconds = expires_in?;
    let usable = if seconds > 5 { seconds - 5 } else { seconds };
    if usable == 0 {
        return Some(now);
    }
    now.checked_add(Duration::from_secs(usable))
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

/// The request URL without query, fragment or userinfo, for display in entries.
pub(crate) fn display_url(url: &Url) -> String {
    let mut shown = url.clone();
    shown.set_query(None);
    shown.set_fragment(None);
    let _ = shown.set_username("");
    let _ = shown.set_password(None);
    shown.to_string()
}

/// The allow-listed response headers, several lines of one header joined by `, `.
pub(crate) fn storable_headers(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    STORED_HEADERS
        .iter()
        .filter_map(|name| {
            let values: Vec<&str> = headers
                .get_all(*name)
                .iter()
                .filter_map(|value| value.to_str().ok())
                .collect();
            (!values.is_empty()).then(|| ((*name).to_owned(), values.join(", ")))
        })
        .collect()
}

/// Apply the storage rules to a response's headers.
pub(crate) fn decide_storage(
    headers: &reqwest::header::HeaderMap,
    authenticated: bool,
    allow_authenticated: bool,
) -> Result<(), NotStored> {
    let lines: Vec<&str> = headers
        .get_all(CACHE_CONTROL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect();
    storage_decision(
        200,
        &CacheControl::parse(&lines),
        header_str(headers, VARY),
        authenticated,
        allow_authenticated,
    )
}

/// Take validators and cache directives from a `304` into the stored entry.
fn merge_validated_headers(meta: &mut CacheEntryMeta, headers: &reqwest::header::HeaderMap) {
    for (name, value) in storable_headers(headers) {
        if matches!(
            name.as_str(),
            "etag" | "last-modified" | "cache-control" | "vary"
        ) && libtea_core::http_cache::is_storable_header(&name, &value)
        {
            meta.headers.retain(|(existing, _)| existing != &name);
            meta.headers.push((name, value));
        }
    }
}

/// A `200` response rebuilt from a cache entry.
fn raw_from_entry(url: &Url, meta: &CacheEntryMeta, body: Vec<u8>) -> RawResponse {
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &meta.headers {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::from_bytes(name.as_bytes()),
            reqwest::header::HeaderValue::from_str(value),
        ) {
            headers.append(name, value);
        }
    }
    RawResponse {
        status: StatusCode::OK,
        headers,
        body: Bytes::from(body),
        final_url: url.clone(),
        content_decoded: meta.decoded,
        pending_cache: None,
        from_cache: None,
        presented_bearer: None,
    }
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
    // Server text: escape control characters before it reaches an error value.
    ClientError::Api {
        status: response.status.as_u16(),
        tea_error: tea.map(|code| libtea_core::escape_control_chars(&code)),
        message: message.map(|text| libtea_core::escape_control_chars(&text)),
    }
}

pub(crate) fn operation_url(
    prefix: &Url,
    path: &str,
    query: &[(&str, &str)],
) -> Result<Url, ClientError> {
    let mut url = prefix.join(path.trim_start_matches('/')).map_err(|_| {
        ClientError::Model(libtea_core::ModelError::Invalid("operation URL".into()))
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

/// Log and error text must not keep query values (tokens, signed URLs, filters).
pub(crate) fn redact_url_for_log(url: &Url) -> String {
    let mut safe = url.clone();
    let _ = safe.set_username("");
    let _ = safe.set_password(None);
    safe.set_fragment(None);
    if url.query().is_none() {
        return safe.to_string();
    }
    let keys: Vec<String> = url.query_pairs().map(|(k, _)| k.into_owned()).collect();
    {
        let mut pairs = safe.query_pairs_mut();
        pairs.clear();
        for key in keys {
            pairs.append_pair(&key, "redacted");
        }
    }
    safe.to_string()
}

/// Sanitize free-form transport error text.
///
/// Structured URL removal happens in [`ClientError::transport`] via
/// `reqwest::Error::without_url`. This helper only rewrites residual absolute
/// HTTP(S) spans. Each span is the full whitespace-bounded token starting at
/// the scheme. That token is rewritten with [`redact_url_for_log`] only when
/// `Url::parse` accepts the entire token. Otherwise the whole message becomes
/// a generic `"request failed"` — partial rewrites must not leave query-value
/// fragments (for example after a quote bound or after trimming `!`).
pub(crate) fn sanitize_transport_text(text: &str) -> String {
    match try_redact_sensitive_text(text) {
        Some(redacted) => redacted,
        None => "request failed".into(),
    }
}

/// Rewrite residual absolute HTTP(S) URLs, or `None` when a span is ambiguous.
pub(crate) fn try_redact_sensitive_text(text: &str) -> Option<String> {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = next_url_start(rest) {
        output.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find(char::is_whitespace).unwrap_or(after.len());
        let token = &after[..end];
        match Url::parse(token) {
            Ok(url) if url.scheme() == "http" || url.scheme() == "https" => {
                output.push_str(&redact_url_for_log(&url));
                rest = &after[end..];
            }
            _ => return None,
        }
    }
    output.push_str(rest);
    Some(output)
}

fn next_url_start(text: &str) -> Option<usize> {
    match (text.find("http://"), text.find("https://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod token_expiry_tests {
    use std::time::{Duration, Instant};

    use super::token_expires_at;

    #[test]
    fn expiry_margin_does_not_consume_a_short_lifetime() {
        let now = Instant::now();
        assert_eq!(token_expires_at(None, now), None);
        assert_eq!(token_expires_at(Some(0), now), Some(now));
        assert_eq!(
            token_expires_at(Some(4), now),
            Some(now + Duration::from_secs(4))
        );
        assert_eq!(
            token_expires_at(Some(5), now),
            Some(now + Duration::from_secs(5))
        );
        assert_eq!(
            token_expires_at(Some(3600), now),
            Some(now + Duration::from_secs(3595))
        );
        assert_eq!(token_expires_at(Some(u64::MAX), now), None);
    }
}

#[cfg(test)]
mod url_redaction_tests {
    use super::{redact_url_for_log, sanitize_transport_text, try_redact_sensitive_text};
    use url::Url;

    #[test]
    fn query_values_are_redacted_and_keys_remain() {
        let url = Url::parse(
            "https://tea.example/v1.0.0/artifact/x/latest/download?mediaType=application%2Fjson&pageToken=secret-token",
        )
        .unwrap();
        let redacted = redact_url_for_log(&url);
        assert!(redacted.contains("mediaType=redacted"));
        assert!(redacted.contains("pageToken=redacted"));
        assert!(!redacted.contains("secret-token"));
        assert!(!redacted.contains("application/json"));
        assert!(!redacted.contains("application%2Fjson"));
    }

    #[test]
    fn userinfo_is_stripped_from_logged_urls() {
        let url = Url::parse("https://user:pass@tea.example/v1.0.0/token?grant=secret").unwrap();
        let redacted = redact_url_for_log(&url);
        assert!(!redacted.contains("user"));
        assert!(!redacted.contains("pass"));
        assert!(redacted.contains("grant=redacted"));
        assert!(redacted.starts_with("https://tea.example/"));
    }

    #[test]
    fn urls_without_query_are_unchanged_aside_from_userinfo() {
        let url = Url::parse("https://tea.example/v1.0.0/products").unwrap();
        assert_eq!(
            redact_url_for_log(&url),
            "https://tea.example/v1.0.0/products"
        );
    }

    #[test]
    fn embedded_urls_in_transport_text_are_redacted() {
        let text = "error downloading https://cdn.example/file?sig=abc123&Expires=99 (timeout)";
        let redacted = try_redact_sensitive_text(text).expect("whitespace-bounded URL");
        assert!(redacted.contains("sig=redacted"));
        assert!(redacted.contains("Expires=redacted"));
        assert!(!redacted.contains("abc123"));
        assert!(redacted.contains("(timeout)"));
    }

    #[test]
    fn earliest_url_is_chosen_when_https_precedes_http() {
        // `find("http://").or_else(|| find("https://"))` would skip the leading
        // https URL and leave `t=two` in the unredacted prefix.
        let text = "fallback https://secure.example/b?t=two then http://plain.example/a?t=one";
        let redacted = try_redact_sensitive_text(text).expect("mixed schemes");
        assert!(redacted.contains("https://secure.example/b?t=redacted"));
        assert!(redacted.contains("http://plain.example/a?t=redacted"));
        assert!(!redacted.contains("t=one"));
        assert!(!redacted.contains("t=two"));
    }

    #[test]
    fn trailing_punctuation_stays_inside_the_query_and_is_redacted() {
        // Sentence punctuation is not peeled off; it is part of the token/query.
        let text = "failed: https://cdn.example/item?token=secret.";
        let redacted = try_redact_sensitive_text(text).expect("period in query");
        assert!(redacted.contains("token=redacted"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("secret."));
    }

    #[test]
    fn quote_inside_query_does_not_leave_a_secret_suffix() {
        // Stopping at `"` used to emit: ...?sig=redacted"SECRET_SUFFIX
        let text = "https://cdn.example/a?sig=prefix\"SECRET_SUFFIX";
        let redacted = try_redact_sensitive_text(text).expect("quote in query");
        assert_eq!(redacted, "https://cdn.example/a?sig=redacted");
        assert!(!redacted.contains("SECRET_SUFFIX"));
        assert!(!redacted.contains("prefix"));
        assert!(!sanitize_transport_text(text).contains("SECRET_SUFFIX"));
    }

    #[test]
    fn trailing_exclamation_marks_stay_in_the_query_and_are_redacted() {
        // Trimming `!` used to emit: ...?sig=redacted!!!
        let text = "https://cdn.example/a?sig=secret!!!";
        let redacted = try_redact_sensitive_text(text).expect("bang in query");
        assert_eq!(redacted, "https://cdn.example/a?sig=redacted");
        assert!(!redacted.contains('!'));
        assert!(!redacted.contains("secret"));
        assert!(!sanitize_transport_text(text).contains("secret"));
    }

    #[test]
    fn commas_semicolons_and_parentheses_do_not_leak_query_secrets() {
        let text = concat!(
            "a=https://cdn.example/a?sig=comma-secret, ",
            "b=(https://cdn.example/b?sig=paren-secret); ",
            "c=https://cdn.example/c?sig=semi-secret; trailing"
        );
        let redacted = try_redact_sensitive_text(text).expect("punctuation in query");
        assert!(!redacted.contains("comma-secret"));
        assert!(!redacted.contains("paren-secret"));
        assert!(!redacted.contains("semi-secret"));
        assert_eq!(redacted.matches("sig=redacted").count(), 3);
        assert!(redacted.contains("trailing"));
    }

    #[test]
    fn fragments_are_stripped_and_query_secrets_are_redacted() {
        let url = Url::parse("https://cdn.example/item?token=frag-secret#section-name").unwrap();
        let redacted = redact_url_for_log(&url);
        assert!(redacted.contains("token=redacted"));
        assert!(!redacted.contains("frag-secret"));
        assert!(!redacted.contains("section-name"));
        assert!(!redacted.contains('#'));

        let text = "see https://cdn.example/item?token=frag-secret#section-name please";
        let embedded = try_redact_sensitive_text(text).expect("fragment token");
        assert!(!embedded.contains("frag-secret"));
        assert!(!embedded.contains("section-name"));
        assert!(embedded.contains("token=redacted"));
        assert!(embedded.ends_with(" please"));
    }

    #[test]
    fn mixed_schemes_and_delimiters_redact_every_query_value() {
        let text = concat!(
            "https://one.example/x?a=one-secret, ",
            "http://two.example/y?b=two-secret; ",
            "(https://three.example/z?c=three-secret)."
        );
        let redacted = try_redact_sensitive_text(text).expect("mixed tokens");
        assert!(!redacted.contains("one-secret"));
        assert!(!redacted.contains("two-secret"));
        assert!(!redacted.contains("three-secret"));
        assert!(redacted.contains("https://one.example/x?a=redacted"));
        assert!(redacted.contains("http://two.example/y?b=redacted"));
        assert!(redacted.contains("https://three.example/z?c=redacted"));
    }

    /// The redaction and challenge parsers slice `str` by byte offsets from
    /// `find`; every combination of multi-byte and delimiter pieces must stay
    /// on character boundaries.
    #[test]
    fn text_helpers_do_not_panic_on_unicode() {
        const PIECES: [&str; 14] = [
            "http://",
            "https://",
            "a",
            "?",
            "=",
            "&",
            " ",
            ",",
            "\"",
            "\u{e9}",
            "\u{20ac}",
            "\u{1f600}",
            "\u{2003}",
            "%",
        ];
        let mut inputs = vec![String::new()];
        for _ in 0..4 {
            let mut next = Vec::new();
            for prefix in &inputs {
                for piece in PIECES {
                    next.push(format!("{prefix}{piece}"));
                }
            }
            inputs.extend(next);
            inputs.dedup();
        }
        for text in &inputs {
            let _ = sanitize_transport_text(text);
            let _ = try_redact_sensitive_text(text);
            let _ = super::challenge_error(text);
            let _ = super::is_bearer_challenge(text);
            if let Ok(url) = Url::parse(text) {
                let _ = redact_url_for_log(&url);
            }
        }
        assert!(inputs.len() > 10_000);
    }

    #[test]
    fn ambiguous_residual_url_falls_back_to_a_generic_message() {
        // A scheme match whose whitespace token is not a parseable absolute URL
        // must not be partially rewritten.
        let text = "broken http://";
        assert!(try_redact_sensitive_text(text).is_none());
        assert_eq!(sanitize_transport_text(text), "request failed");
    }
}

#[cfg(test)]
mod refresh_slot_tests {
    use std::time::Duration;

    use super::{ApiKey, ClientBuilder, ClientError};
    use url::Url;

    fn slot_count(client: &crate::Client) -> usize {
        client
            .inner
            .token_refresh
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .len()
    }

    /// Many distinct origins must not leave one lock entry each behind.
    #[tokio::test]
    async fn refresh_slots_do_not_grow_with_origin_churn() {
        let client = ClientBuilder::new()
            .api_key(ApiKey::new("id", "secret"))
            .build()
            .unwrap();
        for index in 0..500 {
            let prefix = Url::parse(&format!("https://h{index}.example/v1.0.0/")).unwrap();
            // Not an authorized origin, so the exchange fails before any I/O.
            let error = client
                .inner
                .refresh_token_coalesced(&prefix, None)
                .await
                .unwrap_err();
            assert!(matches!(error, ClientError::AuthenticationRequired));
        }
        assert_eq!(slot_count(&client), 0);
    }

    /// A refresh that is cancelled mid-exchange removes its entry too.
    #[tokio::test]
    async fn a_cancelled_refresh_leaves_no_slot() {
        // Accepts connections and never answers.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let hold = tokio::spawn(async move {
            let mut open = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                open.push(socket);
            }
        });
        let origin = Url::parse(&format!("http://{address}")).unwrap();
        let client = ClientBuilder::new()
            .api_key(ApiKey::new("id", "secret"))
            .destination_policy(libtea_core::DestinationPolicy::allow_origin(&origin).unwrap())
            .build()
            .unwrap();
        let prefix = origin.join("v1.0.0/").unwrap();
        client.authorize_origin(&prefix).unwrap();
        let inner = std::sync::Arc::clone(&client.inner);
        let task = tokio::spawn(async move {
            let _ = inner.refresh_token_coalesced(&prefix, None).await;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(slot_count(&client), 1, "the exchange is in flight");
        task.abort();
        let _ = task.await;
        assert_eq!(slot_count(&client), 0);
        hold.abort();
    }
}
