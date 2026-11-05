//! Token exchange against a local mock. No external authorization server.

use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use cdx_tea_client::{ApiKey, Client, ClientError, Limits};
use cdx_tea_core::{DestinationPolicy, ProductId, Tei};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex, Notify};
use url::Url;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PRODUCT_ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const KEY_ID: &str = "id:ent";
const KEY_SECRET: &str = "p@ss:word";

fn limits() -> Limits {
    Limits {
        initial_backoff: Duration::ZERO,
        max_retries: 0,
        request_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(2),
        ..Limits::default()
    }
}

fn client(server: &MockServer, key: bool) -> Client {
    let url = Url::parse(&server.uri()).unwrap();
    let mut builder = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap());
    if key {
        builder = builder.api_key(ApiKey::new(KEY_ID, KEY_SECRET));
    }
    builder.build().unwrap()
}

fn product_json() -> String {
    format!(
        r#"{{"uuid":"{PRODUCT_ID}","name":"Widget","identifiers":[{{"idType":"PURL","idValue":"pkg:generic/widget@1"}}]}}"#
    )
}

fn basic_header(id: &str, secret: &str) -> String {
    let id: String = url::form_urlencoded::byte_serialize(id.as_bytes()).collect();
    let secret: String = url::form_urlencoded::byte_serialize(secret.as_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{id}:{secret}"));
    format!("Basic {encoded}")
}

fn challenge() -> ResponseTemplate {
    ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\"")
}

async fn mount_product(server: &MockServer, token_body: &str, bearer: &str) {
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string("grant_type=client_credentials"))
        .and(header("authorization", basic_header(KEY_ID, KEY_SECRET)))
        .respond_with(ResponseTemplate::new(200).set_body_raw(token_body, "application/json"))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .and(header("authorization", format!("Bearer {bearer}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(product_json(), "application/json"))
        .with_priority(1)
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .with_priority(2)
        .mount(server)
        .await;
}

#[tokio::test]
async fn client_credentials_uses_basic_and_a_form_body() {
    let server = MockServer::start().await;
    mount_product(
        &server,
        r#"{"access_token":"access-token-value","token_type":"Bearer","expires_in":3600,"scope":"tea"}"#,
        "access-token-value",
    )
    .await;
    let product = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap();
    assert_eq!(product.name, "Widget");
    let requests = server.received_requests().await.unwrap();
    let token = requests
        .iter()
        .find(|request| request.url.path().ends_with("/token"))
        .expect("token request");
    assert_eq!(token.method, "POST");
    assert_eq!(token.url.path(), "/v1.0.0/token");
    assert_eq!(&token.body[..], b"grant_type=client_credentials");
    let body = std::str::from_utf8(&token.body).unwrap();
    assert!(!body.contains("scope"));
    assert!(!body.contains("audience"));
    let retry = requests
        .iter()
        .find(|request| {
            request
                .url
                .path()
                .ends_with(&format!("/product/{PRODUCT_ID}"))
        })
        .and_then(|_| {
            requests
                .iter()
                .rev()
                .find(|request| request.url.path().contains("/product/"))
        })
        .expect("retried product request");
    let authorization = retry
        .headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    assert_eq!(authorization, Some("Bearer access-token-value"));
}

#[tokio::test]
async fn short_lifetime_is_still_presented() {
    let server = MockServer::start().await;
    mount_product(
        &server,
        r#"{"access_token":"short-lived","token_type":"bearer","expires_in":4}"#,
        "short-lived",
    )
    .await;
    client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn zero_lifetime_is_not_presented() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"already-dead","token_type":"Bearer","expires_in":0}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .expect(2)
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Api { status: 401, .. }));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[2].headers.get("authorization").is_none());
}

#[tokio::test]
async fn invalid_client_is_one_token_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"error":"invalid_client","error_description":"rejected key-secret"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    match error {
        ClientError::Token { error } => assert_eq!(error, "invalid_client"),
        other => panic!("unexpected {other}"),
    }
}

#[tokio::test]
async fn token_error_text_omits_the_secret() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(
            r#"{"error":"invalid_client","error_description":"rejected key-secret"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    let text = error.to_string();
    assert!(!text.contains(KEY_SECRET));
    assert!(!text.contains("key-secret"));
    assert!(!text.contains("access-token"));
}

#[tokio::test]
async fn rejected_bearer_is_dropped_after_one_retry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"rejected-token","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .and(header("authorization", "Bearer rejected-token"))
        .respond_with(ResponseTemplate::new(401).insert_header(
            "www-authenticate",
            "Bearer realm=\"tea\", error=\"invalid_token\"",
        ))
        .with_priority(1)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .with_priority(2)
        .expect(2)
        .mount(&server)
        .await;
    let service = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap();
    let id = ProductId::parse(PRODUCT_ID).unwrap();
    let first = service.get_product(id).await.unwrap_err();
    let second = service.get_product(id).await.unwrap_err();
    assert!(matches!(first, ClientError::Api { status: 401, .. }));
    assert!(matches!(second, ClientError::Api { status: 401, .. }));
    assert!(!first.to_string().contains("rejected-token"));
    let requests = server.received_requests().await.unwrap();
    let posts = requests
        .iter()
        .filter(|request| request.method == "POST")
        .count();
    assert_eq!(posts, 2);
    let fourth = &requests[3];
    assert_eq!(fourth.method, "GET");
    assert!(fourth.headers.get("authorization").is_none());
}

#[tokio::test]
async fn malformed_token_responses_do_not_panic() {
    let bodies = [
        "{}",
        r#"{"access_token":"","token_type":"Bearer"}"#,
        r#"{"access_token":"tok","token_type":"mac"}"#,
        r#"{"access_token":"tok","token_type":"Bearer","expires_in":"soon"}"#,
        r#"{"access_token":"tok","token_type":"Bearer","expires_in":-1}"#,
    ];
    for body in bodies {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1.0.0/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
            .respond_with(challenge())
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server, true)
            .service(&server.uri(), "1.0.0")
            .unwrap()
            .get_product(ProductId::parse(PRODUCT_ID).unwrap())
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Json(_) | ClientError::Model(_)),
            "{error}"
        );
        assert!(!error.to_string().contains(KEY_SECRET));
    }
}

#[tokio::test]
async fn token_endpoint_errors_are_single_attempts() {
    let cases = [
        (500, r#"{"error":"server_error"}"#, "server_error"),
        (401, "not-json", "401"),
    ];
    for (status, body, expected) in cases {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1.0.0/token"))
            .respond_with(ResponseTemplate::new(status).set_body_raw(body, "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
            .respond_with(challenge())
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server, true)
            .service(&server.uri(), "1.0.0")
            .unwrap()
            .get_product(ProductId::parse(PRODUCT_ID).unwrap())
            .await
            .unwrap_err();
        match error {
            ClientError::Token { error } => assert_eq!(error, expected),
            other => panic!("unexpected {other}"),
        }
    }
}

#[tokio::test]
async fn token_503_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    match error {
        ClientError::Token { error } => assert_eq!(error, "503"),
        other => panic!("unexpected {other}"),
    }
}

#[tokio::test]
async fn token_redirect_is_not_followed() {
    let server = MockServer::start().await;
    let elsewhere = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/steal", elsewhere.uri())),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&elsewhere)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(challenge())
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Token { .. }));
}

#[tokio::test]
async fn insufficient_scope_does_not_fetch_a_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(ResponseTemplate::new(401).insert_header(
            "www-authenticate",
            "Bearer realm=\"tea\", error=\"insufficient_scope\"",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server, true)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::AuthenticationRequired));
}

#[tokio::test]
async fn discovery_does_not_post_the_key_unless_allowed() {
    let server = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(challenge())
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"should-not-be-requested","token_type":"Bearer"}"#,
            "application/json",
        ))
        .expect(0)
        .mount(&server)
        .await;
    let url = Url::parse(&server.uri()).unwrap();
    let error = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .api_key(ApiKey::new(KEY_ID, KEY_SECRET))
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", server.uri())).unwrap())
        .build()
        .unwrap()
        .discover(&tei)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::AuthenticationRequired));
}

#[tokio::test]
async fn discovered_credentials_are_posted_only_when_enabled() {
    let server = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"discovered-token","token_type":"Bearer","expires_in":60}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .and(header("authorization", "Bearer discovered-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "productReleaseUuid": "d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
                "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"]}]
            }])),
        )
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(challenge())
        .with_priority(2)
        .mount(&server)
        .await;
    let url = Url::parse(&server.uri()).unwrap();
    let found = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .api_key(ApiKey::new(KEY_ID, KEY_SECRET))
        .allow_discovered_credentials(true)
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", server.uri())).unwrap())
        .build()
        .unwrap()
        .discover(&tei)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
}

const STALE_TOKEN: &str = "token-stale";
const FRESH_TOKEN: &str = "token-fresh";

const SHARED_TOKEN: &str = "shared-token";

/// Under a forced two-challenge gate, concurrent refreshes share one `POST /token`.
///
/// Both callers are forced to observe a challenge before the token endpoint
/// answers, so the assertion does not depend on scheduling luck. This does not
/// claim every concurrent workload yields exactly one exchange.
#[tokio::test]
async fn concurrent_challenges_share_one_token_exchange() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let gate = Arc::new(CoalesceGate {
        log: Mutex::new(Vec::new()),
        challenges: Mutex::new(0u32),
        both_challenged: Notify::new(),
        issued: Mutex::new(0u32),
    });
    tokio::spawn({
        let gate = Arc::clone(&gate);
        async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let gate = Arc::clone(&gate);
                tokio::spawn(async move {
                    let _ = serve_coalesce(socket, &gate).await;
                });
            }
        }
    });

    let base = format!("http://{addr}");
    let url = Url::parse(&base).unwrap();
    let service = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .api_key(ApiKey::new(KEY_ID, KEY_SECRET))
        .build()
        .unwrap()
        .service(&base, "1.0.0")
        .unwrap();
    let id = ProductId::parse(PRODUCT_ID).unwrap();
    let left = {
        let service = service.clone();
        tokio::spawn(async move { service.get_product(id).await })
    };
    let right = {
        let service = service.clone();
        tokio::spawn(async move { service.get_product(id).await })
    };
    let left = left.await.unwrap().unwrap();
    let right = right.await.unwrap().unwrap();
    assert_eq!(left.name, "Widget");
    assert_eq!(right.name, "Widget");

    let log = gate.log.lock().await;
    let posts = log.iter().filter(|(method, _, _)| method == "POST").count();
    assert_eq!(
        posts, 1,
        "expected one coalesced token exchange, log={log:?}"
    );
    assert_eq!(*gate.issued.lock().await, 1);
    let challenges = log
        .iter()
        .filter(|(method, path, auth)| {
            method == "GET" && path.contains("/product/") && auth.is_none()
        })
        .count();
    assert_eq!(challenges, 2);
}

/// A delayed 401 for an older bearer token must not delete a newer token
/// stored for the same origin while that response was in flight.
#[tokio::test]
async fn stale_401_keeps_a_newer_cached_token() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (release_tx, release_rx) = oneshot::channel::<()>();
    let release_rx = Arc::new(Mutex::new(Some(release_rx)));
    let held = Arc::new(Notify::new());
    let log = Arc::new(Mutex::new(Vec::<ObservedRequest>::new()));
    let gate = Arc::new(TokenGate {
        release_rx,
        held: Arc::clone(&held),
        log: Arc::clone(&log),
        holding: Mutex::new(false),
        issued: Mutex::new(0u32),
    });
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let gate = Arc::clone(&gate);
            tokio::spawn(async move {
                let _ = serve_token_race(socket, &gate).await;
            });
        }
    });

    let base = format!("http://{addr}");
    let url = Url::parse(&base).unwrap();
    let service = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .api_key(ApiKey::new("race-id", "race-secret"))
        .build()
        .unwrap()
        .service(&base, "1.0.0")
        .unwrap();
    let id = ProductId::parse(PRODUCT_ID).unwrap();
    let stale = {
        let service = service.clone();
        tokio::spawn(async move { service.get_product(id).await })
    };
    held.notified().await;

    let fresh = service.get_product(id).await.unwrap();
    assert_eq!(fresh.name, "Widget");
    release_tx.send(()).unwrap();

    let stale = stale.await.unwrap().unwrap_err();
    assert!(matches!(stale, ClientError::Api { status: 401, .. }));
    assert!(!stale.to_string().contains(STALE_TOKEN));
    assert!(!stale.to_string().contains(FRESH_TOKEN));
    assert!(!stale.to_string().contains("race-secret"));

    let after = service.get_product(id).await.unwrap();
    assert_eq!(after.name, "Widget");

    let log = log.lock().await;
    let posts = log.iter().filter(|(method, _, _)| method == "POST").count();
    assert_eq!(posts, 2, "a stale 401 must not exchange the key again");
    let last_product = log
        .iter()
        .rev()
        .find(|(_, path, _)| path.contains("/product/"))
        .unwrap();
    assert_eq!(
        last_product.2.as_deref(),
        Some(format!("Bearer {FRESH_TOKEN}").as_str())
    );
}

type ObservedRequest = (String, String, Option<String>);

struct CoalesceGate {
    log: Mutex<Vec<ObservedRequest>>,
    challenges: Mutex<u32>,
    both_challenged: Notify,
    issued: Mutex<u32>,
}

async fn serve_coalesce(
    mut socket: tokio::net::TcpStream,
    gate: &CoalesceGate,
) -> std::io::Result<()> {
    let (method, path, authorization) = read_request(&mut socket).await?;
    let recorded = authorization
        .as_deref()
        .filter(|value| value.starts_with("Bearer ") || value.starts_with("Basic "))
        .map(str::to_owned);
    // Log product GETs and token POSTs; Basic values are redacted in asserts.
    let logged_auth = match recorded.as_deref() {
        Some(value) if value.starts_with("Basic ") => Some("Basic <redacted>".to_owned()),
        other => other.map(str::to_owned),
    };
    gate.log
        .lock()
        .await
        .push((method.clone(), path.clone(), logged_auth));

    if method == "POST" && path == "/v1.0.0/token" {
        // Wait until both resource requests have been challenged so a fast
        // first exchange cannot starve the second challenge path. Subscribe
        // before reading the counter so a notify between check and await is
        // not lost.
        loop {
            let wait = gate.both_challenged.notified();
            if *gate.challenges.lock().await >= 2 {
                break;
            }
            wait.await;
        }
        *gate.issued.lock().await += 1;
        let body = format!(
            r#"{{"access_token":"{SHARED_TOKEN}","token_type":"Bearer","expires_in":3600}}"#
        );
        return write_response(
            &mut socket,
            200,
            &[("content-type", "application/json")],
            &body,
        )
        .await;
    }

    if method == "GET" && path.contains("/product/") {
        match authorization.as_deref() {
            Some(value) if value == format!("Bearer {SHARED_TOKEN}") => {
                return write_response(
                    &mut socket,
                    200,
                    &[("content-type", "application/json")],
                    &product_json(),
                )
                .await;
            }
            _ => {
                {
                    let mut challenges = gate.challenges.lock().await;
                    *challenges += 1;
                    if *challenges >= 2 {
                        gate.both_challenged.notify_waiters();
                    }
                }
                return write_response(
                    &mut socket,
                    401,
                    &[("www-authenticate", "Bearer realm=\"tea\"")],
                    "",
                )
                .await;
            }
        }
    }

    write_response(&mut socket, 404, &[], "").await
}

struct TokenGate {
    release_rx: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    held: Arc<Notify>,
    log: Arc<Mutex<Vec<ObservedRequest>>>,
    holding: Mutex<bool>,
    issued: Mutex<u32>,
}

async fn serve_token_race(
    mut socket: tokio::net::TcpStream,
    gate: &TokenGate,
) -> std::io::Result<()> {
    let (method, path, authorization) = read_request(&mut socket).await?;
    let recorded = authorization
        .as_deref()
        .filter(|value| value.starts_with("Bearer "))
        .map(str::to_owned);
    gate.log
        .lock()
        .await
        .push((method.clone(), path.clone(), recorded));

    if method == "POST" && path == "/v1.0.0/token" {
        let mut issued = gate.issued.lock().await;
        *issued += 1;
        let token = if *issued == 1 {
            STALE_TOKEN
        } else {
            FRESH_TOKEN
        };
        let body =
            format!(r#"{{"access_token":"{token}","token_type":"Bearer","expires_in":3600}}"#);
        return write_response(
            &mut socket,
            200,
            &[("content-type", "application/json")],
            &body,
        )
        .await;
    }

    if method == "GET" && path.contains("/product/") {
        match authorization.as_deref() {
            Some(value) if value == format!("Bearer {FRESH_TOKEN}") => {
                return write_response(
                    &mut socket,
                    200,
                    &[("content-type", "application/json")],
                    &product_json(),
                )
                .await;
            }
            Some(value) if value == format!("Bearer {STALE_TOKEN}") => {
                let hold = {
                    let mut holding = gate.holding.lock().await;
                    if *holding {
                        false
                    } else {
                        *holding = true;
                        true
                    }
                };
                if hold {
                    gate.held.notify_one();
                    let release = gate.release_rx.lock().await.take();
                    if let Some(release) = release {
                        let _ = release.await;
                    }
                }
                return write_response(
                    &mut socket,
                    401,
                    &[(
                        "www-authenticate",
                        "Bearer realm=\"tea\", error=\"invalid_token\"",
                    )],
                    "",
                )
                .await;
            }
            _ => {
                return write_response(
                    &mut socket,
                    401,
                    &[("www-authenticate", "Bearer realm=\"tea\"")],
                    "",
                )
                .await;
            }
        }
    }

    write_response(&mut socket, 404, &[], "").await
}

async fn read_request(
    socket: &mut tokio::net::TcpStream,
) -> std::io::Result<(String, String, Option<String>)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    let header_end = loop {
        let n = socket.read(&mut tmp).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "request ended before headers",
            ));
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 65_536 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request headers are too large",
            ));
        }
    };
    let header = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = header.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let authorization = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("authorization") {
            Some(value.trim().to_owned())
        } else {
            None
        }
    });
    Ok((method, path, authorization))
}

async fn write_response(
    socket: &mut tokio::net::TcpStream,
    status: u16,
    headers: &[(&str, &str)],
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        _ => "Not Found",
    };
    let mut message = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        message.push_str(name);
        message.push_str(": ");
        message.push_str(value);
        message.push_str("\r\n");
    }
    message.push_str("\r\n");
    message.push_str(body);
    socket.write_all(message.as_bytes()).await?;
    Ok(())
}
