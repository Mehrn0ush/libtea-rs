//! Broader live checks against a local Oolong fixture profile.
//!
//! Ignored by default. Requires Oolong on `127.0.0.1:3005` with the
//! `fixtures/tea-1.0.0` content used by `docker-compose.tea-interop.yml`.
//!
//! ```text
//! cargo test -p cdx-tea-client --test oolong_surface_live -- --ignored --nocapture
//! ```
//!
//! These tests record SDK behavior against one server. They do not certify
//! TEA 1.0.0. Unsupported server responses are asserted as such.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use cdx_tea_client::{ApiKey, ArtifactFetch, CleFetch, Client, ClientError, SignatureFetch};
use cdx_tea_core::enums::{CollectionSort, IdentifierType, ProductSort, ReleaseSort, SortOrder};
use cdx_tea_core::{
    ArtifactId, ComponentId, ComponentReleaseId, IntegrityOutcome, ListParams, ProductId,
    ProductReleaseId, Tei,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

const UPSTREAM: &str = "127.0.0.1:3005";
const PRODUCT: &str = "11111111-1111-4111-8111-111111111111";
const PRODUCT_RELEASE: &str = "22222222-2222-4222-8222-222222222222";
const COMPONENT: &str = "33333333-3333-4333-8333-333333333333";
const COMPONENT_RELEASE: &str = "44444444-4444-4444-8444-444444444444";
const ARTIFACT: &str = "55555555-5555-4555-8555-555555555555";
const UNKNOWN: &str = "00000000-0000-4000-8000-000000000000";

struct Transcript {
    lines: Mutex<Vec<String>>,
}

impl Transcript {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            lines: Mutex::new(Vec::new()),
        })
    }

    fn snapshot(&self) -> Vec<String> {
        self.lines.lock().expect("transcript").clone()
    }
}

struct Live {
    base: String,
    transcript: Arc<Transcript>,
}

async fn live() -> Live {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("proxy bind");
    let port = listener.local_addr().expect("proxy addr").port();
    let transcript = Transcript::new();
    let recorded = Arc::clone(&transcript);
    tokio::spawn(async move {
        loop {
            let Ok((inbound, _)) = listener.accept().await else {
                break;
            };
            let recorded = Arc::clone(&recorded);
            tokio::spawn(async move {
                if let Err(error) = forward(inbound, &recorded, None).await {
                    if error.kind() != std::io::ErrorKind::TimedOut
                        && error.kind() != std::io::ErrorKind::UnexpectedEof
                    {
                        eprintln!("live proxy: {error}");
                    }
                }
            });
        }
    });
    Live {
        base: format!("http://127.0.0.1:{port}"),
        transcript,
    }
}

/// First non-token request gets a Bearer challenge; later requests forward.
async fn live_with_bearer_challenge() -> Live {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("proxy bind");
    let port = listener.local_addr().expect("proxy addr").port();
    let transcript = Transcript::new();
    let recorded = Arc::clone(&transcript);
    let challenged = Arc::new(Mutex::new(false));
    tokio::spawn(async move {
        loop {
            let Ok((inbound, _)) = listener.accept().await else {
                break;
            };
            let recorded = Arc::clone(&recorded);
            let challenged = Arc::clone(&challenged);
            tokio::spawn(async move {
                if let Err(error) =
                    forward_with_optional_challenge(inbound, &recorded, &challenged).await
                {
                    if error.kind() != std::io::ErrorKind::TimedOut
                        && error.kind() != std::io::ErrorKind::UnexpectedEof
                    {
                        eprintln!("challenge proxy: {error}");
                    }
                }
            });
        }
    });
    Live {
        base: format!("http://127.0.0.1:{port}"),
        transcript,
    }
}

async fn forward(
    mut client: TcpStream,
    transcript: &Transcript,
    _unused: Option<()>,
) -> std::io::Result<()> {
    let mut upstream = TcpStream::connect(UPSTREAM).await?;
    let mut pending = Vec::new();
    loop {
        let header_end = match read_headers(&mut client, &mut pending).await {
            Ok(end) => end,
            Err(error)
                if pending.is_empty()
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::UnexpectedEof
                    ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let headers = String::from_utf8_lossy(&pending[..header_end]).into_owned();
        let request_line = headers.lines().next().unwrap_or("").to_owned();
        let head_request = request_line.starts_with("HEAD ");
        transcript
            .lines
            .lock()
            .expect("transcript")
            .push(request_line);
        let total = header_end + content_length(&headers);
        read_exact_at_least(&mut client, &mut pending, total).await?;
        upstream.write_all(&pending[..total]).await?;
        upstream.flush().await?;
        pending.drain(..total);

        let mut response = Vec::new();
        let response_end = read_headers(&mut upstream, &mut response).await?;
        let response_headers = String::from_utf8_lossy(&response[..response_end]).into_owned();
        // HEAD responses may advertise Content-Length for the representation
        // while carrying no message body.
        let response_total = if head_request {
            response_end
        } else {
            response_end + content_length(&response_headers)
        };
        read_exact_at_least(&mut upstream, &mut response, response_total).await?;
        client.write_all(&response[..response_total]).await?;
        client.flush().await?;
        if closes(&headers) || closes(&response_headers) {
            return Ok(());
        }
    }
}

async fn forward_with_optional_challenge(
    mut client: TcpStream,
    transcript: &Transcript,
    challenged: &Mutex<bool>,
) -> std::io::Result<()> {
    let mut pending = Vec::new();
    let header_end = read_headers(&mut client, &mut pending).await?;
    let headers = String::from_utf8_lossy(&pending[..header_end]).into_owned();
    let request_line = headers.lines().next().unwrap_or("").to_owned();
    transcript
        .lines
        .lock()
        .expect("transcript")
        .push(request_line.clone());
    let total = header_end + content_length(&headers);
    read_exact_at_least(&mut client, &mut pending, total).await?;

    // Request-line is `POST /v…/token HTTP/1.1` (method at the start). The
    // earlier `contains(" POST /")` check never matched and could challenge the
    // token exchange itself.
    let is_token = is_token_request_line(&request_line);
    let should_challenge = {
        let mut seen = challenged.lock().expect("challenged");
        if !is_token && !*seen {
            *seen = true;
            true
        } else {
            false
        }
    };
    if should_challenge {
        // Synthetic fixture only: Oolong's tea-1.0.0 profile does not emit this
        // Bearer challenge. The proxy injects it so the SDK exercises POST /token.
        let body = br#"{"error":"invalid_token","message":"synthetic-proxy-bearer-challenge"}"#;
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer error=\"invalid_token\"\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        client.write_all(response.as_bytes()).await?;
        client.write_all(body).await?;
        client.flush().await?;
        return Ok(());
    }

    let mut upstream = TcpStream::connect(UPSTREAM).await?;
    upstream.write_all(&pending[..total]).await?;
    upstream.flush().await?;
    let mut response = Vec::new();
    let response_end = read_headers(&mut upstream, &mut response).await?;
    let response_headers = String::from_utf8_lossy(&response[..response_end]).into_owned();
    let response_total = response_end + content_length(&response_headers);
    read_exact_at_least(&mut upstream, &mut response, response_total).await?;
    client.write_all(&response[..response_total]).await?;
    client.flush().await?;
    Ok(())
}

async fn read_headers(stream: &mut TcpStream, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    loop {
        if let Some(end) = header_end(buf) {
            return Ok(end);
        }
        let mut tmp = [0_u8; 4096];
        let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut tmp))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "header timeout"))??;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof before headers",
            ));
        }
        buf.extend_from_slice(&tmp[..read]);
    }
}

async fn read_exact_at_least(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    total: usize,
) -> std::io::Result<()> {
    while buf.len() < total {
        let mut tmp = [0_u8; 4096];
        let read = tokio::time::timeout(Duration::from_secs(3), stream.read(&mut tmp))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "body timeout"))??;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof before body",
            ));
        }
        buf.extend_from_slice(&tmp[..read]);
    }
    Ok(())
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn closes(headers: &str) -> bool {
    headers.lines().any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.eq_ignore_ascii_case("connection") && value.trim().eq_ignore_ascii_case("close")
    })
}

fn live_policy(proxy_base: &str) -> cdx_tea_core::DestinationPolicy {
    let proxy = Url::parse(proxy_base).expect("proxy origin");
    let upstream = Url::parse(&format!("http://{UPSTREAM}")).expect("upstream origin");
    cdx_tea_core::DestinationPolicy::allow_origin(&proxy)
        .expect("proxy")
        .and_allow(&upstream)
        .expect("upstream")
}

fn service(base: &str) -> cdx_tea_client::Service {
    Client::builder()
        .destination_policy(live_policy(base))
        .well_known_fetch_url(Url::parse(&format!("{base}/.well-known/tea")).expect("wk"))
        .build()
        .expect("client")
        .service(base, "1.0.0")
        .expect("service")
}

fn client_with_key(base: &str) -> Client {
    Client::builder()
        .destination_policy(live_policy(base))
        .api_key(ApiKey::new("live-id", "live-secret"))
        .well_known_fetch_url(Url::parse(&format!("{base}/.well-known/tea")).expect("wk"))
        .build()
        .expect("client")
}

fn params<S>() -> ListParams<S> {
    ListParams::default()
}

/// True when the HTTP request-line is a `POST` to a `/token` path.
fn is_token_request_line(request_line: &str) -> bool {
    let mut parts = request_line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some(method), Some(path)) => {
            method.eq_ignore_ascii_case("POST") && path.contains("/token")
        }
        _ => false,
    }
}

#[test]
fn token_request_line_starts_with_post_slash() {
    assert!(is_token_request_line("POST /v1.0.0/token HTTP/1.1"));
    assert!(is_token_request_line("POST /token HTTP/1.1"));
    assert!(!is_token_request_line(
        "GET /v1.0.0/product/11111111-1111-4111-8111-111111111111 HTTP/1.1"
    ));
    // The defective check required a leading space before POST.
    assert!(!{
        let request_line = "POST /v1.0.0/token HTTP/1.1";
        request_line.contains(" POST /") && request_line.contains("/token")
    });
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn discovery_get_collection_artifact_and_cle() {
    let live = live().await;
    let client = Client::builder()
        .destination_policy(live_policy(&live.base))
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", live.base)).unwrap())
        .build()
        .expect("client");
    let tei = Tei::parse(&format!("tei://localhost/uuid/{PRODUCT_RELEASE}")).unwrap();
    let discovered = client.discover(&tei).await.expect("discover");
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].api_version, "1.0.0");

    let svc = service(&live.base);
    let pid = ProductId::parse(PRODUCT).unwrap();
    let product = svc.get_product(pid).await.expect("product");
    assert_eq!(product.name, "Alpha Fixture");

    let rid = ProductReleaseId::parse(PRODUCT_RELEASE).unwrap();
    let release = svc.get_product_release(rid).await.expect("release");
    assert_eq!(release.release.version, "1.0.0");

    let nested = svc
        .list_product_releases(pid, &params::<ReleaseSort>())
        .await
        .expect("nested product releases");
    assert_eq!(
        nested.results.len(),
        3,
        "Alpha Fixture has three releases in the tea-1.0.0 fixture"
    );
    let nested_versions: Vec<_> = nested
        .results
        .iter()
        .map(|item| item.version.as_str())
        .collect();
    assert!(
        nested_versions.contains(&"0.9.0")
            && nested_versions.contains(&"1.0.0")
            && nested_versions.contains(&"1.1.0"),
        "nested versions: {nested_versions:?}"
    );
    assert!(!nested.has_next);

    let mut collections = params::<CollectionSort>();
    collections.page_size = Some(1);
    collections.sort_field = Some(CollectionSort::Version);
    collections.sort_order = Some(SortOrder::Asc);
    let first = svc
        .list_product_release_collections(rid, &collections)
        .await
        .expect("collections");
    assert_eq!(first.results.len(), 1);
    assert!(first.has_next, "fixture has two collections; pageSize=1");
    assert!(first.next_page_token.is_some());
    let requests = live.transcript.snapshot();
    assert!(
        requests.iter().any(|line| {
            line.contains("/productRelease/")
                && line.contains("/collections?")
                && line.contains("pageSize=1")
                && line.contains("sortField=version")
                && line.contains("sortOrder=asc")
        }),
        "collection list query: {requests:?}"
    );
    collections.page_token = first.next_page_token.clone();
    let second = svc
        .list_product_release_collections(rid, &collections)
        .await
        .expect("collection page 2");
    assert_eq!(second.results.len(), 1);
    assert!(!second.has_next);
    let after = live.transcript.snapshot();
    let last = after.last().expect("second request");
    assert!(
        last.contains("pageSize=1")
            && last.contains("sortField=version")
            && last.contains("sortOrder=asc")
            && last.contains("pageToken="),
        "continuation kept filters: {last}"
    );

    let latest = svc
        .get_latest_product_release_collection(rid)
        .await
        .expect("latest collection");
    assert!(!latest.artifacts.is_empty());
    let versioned = svc
        .get_product_release_collection(rid, latest.version)
        .await
        .expect("versioned collection");
    assert_eq!(versioned.version, latest.version);

    let cid = ComponentId::parse(COMPONENT).unwrap();
    assert_eq!(
        svc.get_component(cid).await.expect("component").name,
        "Fixture Part"
    );
    let nested_components = svc
        .list_component_releases(cid, &params::<ReleaseSort>())
        .await
        .expect("nested component releases");
    assert_eq!(nested_components.results.len(), 2);
    let component_versions: Vec<_> = nested_components
        .results
        .iter()
        .map(|item| item.version.as_str())
        .collect();
    assert!(
        component_versions.contains(&"1.0.0") && component_versions.contains(&"1.1.0"),
        "component releases: {component_versions:?}"
    );
    assert!(!nested_components.has_next);

    let crid = ComponentReleaseId::parse(COMPONENT_RELEASE).unwrap();
    let crel = svc.get_component_release(crid).await.expect("crelease");
    assert_eq!(crel.release.version, "1.0.0");

    let mut component_collections = params::<CollectionSort>();
    component_collections.page_size = Some(1);
    component_collections.sort_field = Some(CollectionSort::Version);
    component_collections.sort_order = Some(SortOrder::Asc);
    let c_first = svc
        .list_component_release_collections(crid, &component_collections)
        .await
        .expect("component collections");
    assert_eq!(c_first.results.len(), 1);
    assert!(
        c_first.has_next,
        "fixture has two component-release collections; pageSize=1"
    );
    component_collections.page_token = c_first.next_page_token.clone();
    let c_second = svc
        .list_component_release_collections(crid, &component_collections)
        .await
        .expect("component collection page 2");
    assert_eq!(c_second.results.len(), 1);
    assert!(!c_second.has_next);

    let c_latest = svc
        .get_latest_component_release_collection(crid)
        .await
        .expect("component collection");
    assert_eq!(c_latest.version, 2);
    assert_eq!(
        c_latest.artifacts.len(),
        1,
        "latest component-release collection lists one artifact"
    );
    let c_v1 = svc
        .get_component_release_collection(crid, 1)
        .await
        .expect("component collection v1");
    assert_eq!(c_v1.version, 1);
    assert!(
        c_v1.artifacts.is_empty(),
        "fixture collection version 1 has no artifacts"
    );
    let c_v2 = svc
        .get_component_release_collection(crid, 2)
        .await
        .expect("component collection v2");
    assert_eq!(c_v2.version, 2);
    assert_eq!(c_v2.artifacts.len(), 1);

    let aid = ArtifactId::parse(ARTIFACT).unwrap();
    let latest_meta = svc.get_latest_artifact(aid).await.expect("artifact latest");
    assert_eq!(latest_meta.version, 2);
    let v1 = svc.get_artifact(aid, 1).await.expect("artifact v1");
    assert_eq!(v1.version, 1);
    let checksums = latest_meta
        .formats
        .first()
        .and_then(|format| format.checksums.clone())
        .unwrap_or_default();
    match svc
        .download_latest_artifact(aid, None, &checksums, None)
        .await
        .expect("download latest")
    {
        ArtifactFetch::Body(body) => {
            assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
            assert!(!body.bytes.is_empty());
        }
        other => panic!("expected body: {other:?}"),
    }
    // Version 1 in this fixture points at an external URL. Download the hosted
    // version-2 path instead so the check stays on Oolong, not a third host.
    let v2_checksums = latest_meta
        .formats
        .first()
        .and_then(|format| format.checksums.clone())
        .unwrap_or_default();
    match svc
        .download_artifact(aid, 2, None, &v2_checksums, None)
        .await
        .expect("download v2")
    {
        ArtifactFetch::Body(body) => {
            assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
            assert!(!body.bytes.is_empty());
        }
        other => panic!("expected body: {other:?}"),
    }

    let headers = svc
        .head_artifact(aid, 2, Some("application/vnd.cyclonedx+json"))
        .await
        .expect("head");
    assert!(
        headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("etag")
                || name.eq_ignore_ascii_case("content-length")),
        "head headers: {headers:?}"
    );

    match svc.get_product_cle(pid).await.expect("product cle") {
        CleFetch::NotImplemented { .. } => {}
        other => panic!("Oolong product CLE unsupported: {other:?}"),
    }
    match svc.get_product_release_cle(rid).await.expect("release cle") {
        CleFetch::NotImplemented { .. } => {}
        other => panic!("Oolong product-release CLE unsupported: {other:?}"),
    }
    match svc.get_component_cle(cid).await.expect("component cle") {
        CleFetch::NotImplemented { .. } => {}
        other => panic!("Oolong component CLE unsupported: {other:?}"),
    }
    match svc
        .get_component_release_cle(crid)
        .await
        .expect("component-release cle")
    {
        CleFetch::NotImplemented { .. } => {}
        other => panic!("Oolong component-release CLE unsupported: {other:?}"),
    }

    match svc
        .download_latest_signature(aid, None, None)
        .await
        .expect("latest signature")
    {
        SignatureFetch::Present(body) => {
            // Bytes are returned. This is not cryptographic signature verification.
            assert!(!body.bytes.is_empty());
        }
        other => panic!("expected signature bytes from Oolong download path: {other:?}"),
    }
    match svc
        .download_signature(aid, 2, None, None)
        .await
        .expect("versioned signature")
    {
        SignatureFetch::Present(body) => {
            // Versioned download only. Bytes are not cryptographically verified.
            assert!(!body.bytes.is_empty());
            assert!(
                body.bytes.starts_with(b"fixture-detached-signature-v1"),
                "signature bytes: {:?}",
                String::from_utf8_lossy(&body.bytes)
            );
        }
        other => panic!("expected versioned signature bytes: {other:?}"),
    }
    match svc
        .download_signature(aid, 1, None, None)
        .await
        .expect("missing versioned signature")
    {
        SignatureFetch::NotPublished => {}
        other => panic!("fixture revision 1 has no signature: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn sorted_filtered_product_continuation_keeps_query() {
    let live = live().await;
    let svc = service(&live.base);
    let mut query = params::<ProductSort>();
    query.page_size = Some(1);
    query.sort_field = Some(ProductSort::Name);
    query.sort_order = Some(SortOrder::Asc);
    query.id_type = Some(IdentifierType::Purl);
    let first = svc.query_products(&query).await.expect("first");
    assert!(first.next_page_token.is_some());
    query.page_token = first.next_page_token.clone();
    let second = svc.query_products(&query).await.expect("second");
    assert!(!second.results.is_empty());
    let requests = live.transcript.snapshot();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert!(
        requests[1].contains("idType=PURL")
            && requests[1].contains("pageSize=1")
            && requests[1].contains("sortField=name")
            && requests[1].contains("sortOrder=asc")
            && requests[1].contains("pageToken="),
        "continuation: {}",
        requests[1]
    );
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn unknown_objects_map_to_api_object_unknown() {
    let live = live().await;
    let svc = service(&live.base);
    let err = svc
        .get_product(ProductId::parse(UNKNOWN).unwrap())
        .await
        .expect_err("missing product");
    match err {
        ClientError::Api {
            status: 404,
            tea_error: Some(code),
            ..
        } => assert_eq!(code, "OBJECT_UNKNOWN"),
        other => panic!("expected OBJECT_UNKNOWN: {other}"),
    }
}

#[tokio::test]
#[ignore = "live Oolong on 127.0.0.1:3005"]
async fn token_endpoint_not_implemented_is_server_limitation() {
    // Distinction: the Bearer 401 is injected by `live_with_bearer_challenge`
    // (synthetic proxy). Oolong's tea-1.0.0 fixture does not challenge callers
    // that way; once forced, its POST /token returns NOT_IMPLEMENTED. That is a
    // server limitation, not an SDK defect.
    let live = live_with_bearer_challenge().await;
    let client = client_with_key(&live.base);
    let svc = client.service(&live.base, "1.0.0").expect("service");
    let err = svc
        .get_product(ProductId::parse(PRODUCT).unwrap())
        .await
        .expect_err("token exchange must fail");
    match err {
        ClientError::Api {
            status: 404,
            tea_error: Some(code),
            ..
        } => assert_eq!(code, "NOT_IMPLEMENTED"),
        other => panic!("expected token NOT_IMPLEMENTED mapping: {other}"),
    }
    let requests = live.transcript.snapshot();
    let get_at = requests
        .iter()
        .position(|line| line.starts_with("GET /") && line.contains("/product/"))
        .unwrap_or_else(|| panic!("challenged GET observed: {requests:?}"));
    let token_at = requests
        .iter()
        .position(|line| is_token_request_line(line))
        .unwrap_or_else(|| panic!("token POST observed: {requests:?}"));
    assert!(
        get_at < token_at,
        "synthetic challenge precedes token POST: {requests:?}"
    );
    assert!(
        requests[token_at].starts_with("POST /"),
        "token request-line must begin with POST /: {}",
        requests[token_at]
    );
}
