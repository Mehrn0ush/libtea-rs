//! HTTP behavior against a local mock. These tests do not use the public network.

use std::time::Duration;

use libtea_client::{ApiKey, ArtifactFetch, Client, ClientError, Limits};
use libtea_core::{Checksum, ChecksumAlgorithm, DestinationPolicy, ProductId, Tei};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PRODUCT_ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn limits() -> Limits {
    Limits::default()
        .with_initial_backoff(Duration::ZERO)
        .with_max_retries(0)
        .with_request_timeout(Duration::from_secs(5))
        .with_connect_timeout(Duration::from_secs(2))
}

fn client_for(server: &MockServer) -> Client {
    let url = Url::parse(&server.uri()).unwrap();
    Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .build()
        .unwrap()
}

fn product_json() -> String {
    format!(
        r#"{{"uuid":"{PRODUCT_ID}","name":"Widget","identifiers":[{{"idType":"PURL","idValue":"pkg:generic/widget@1"}}]}}"#
    )
}

#[tokio::test]
async fn production_policy_rejects_cleartext_and_metadata() {
    let client = Client::new().unwrap();
    assert!(client.service("http://example.com", "1.0.0").is_err());
    assert!(client.service("https://169.254.169.254", "1.0.0").is_err());
    assert!(client.service("https://api.example.com", "1.0.0").is_ok());
}

#[tokio::test]
async fn gets_a_product() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(product_json(), "application/json"))
        .mount(&server)
        .await;
    let service = client_for(&server).service(&server.uri(), "1.0.0").unwrap();
    let product = service
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap();
    assert_eq!(product.name, "Widget");
}

#[tokio::test]
async fn discovery_follows_version_and_stops_on_tea_404() {
    let server = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    let root = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": root, "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .and(query_param("tei", tei.as_str()))
        .respond_with(ResponseTemplate::new(404).set_body_raw(
            r#"{"error":"OBJECT_UNKNOWN","message":"withheld or unknown"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let url = Url::parse(&format!("{root}/.well-known/tea")).unwrap();
    let client = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&Url::parse(&root).unwrap()).unwrap())
        .well_known_fetch_url(url)
        .build()
        .unwrap();
    let error = client.discover(&tei).await.unwrap_err();
    match error {
        ClientError::Unresolved { error, .. } => assert_eq!(error, "OBJECT_UNKNOWN"),
        other => panic!("unexpected {other}"),
    }
}

#[tokio::test]
async fn non_tea_404_fails_over_to_the_next_endpoint() {
    let bad = MockServer::start().await;
    let good = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    let policy = DestinationPolicy::allow_origin(&Url::parse(&bad.uri()).unwrap())
        .unwrap()
        .and_allow(&Url::parse(&good.uri()).unwrap())
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [
                {"url": bad.uri(), "versions": ["1.0.0"], "priority": 1},
                {"url": good.uri(), "versions": ["1.0.0"], "priority": 0.2}
            ]
        })))
        .mount(&good)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
        .expect(1)
        .mount(&bad)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(serde_json::json!([{
                    "productReleaseUuid": "d4d9f54a-abcf-11ee-ac79-1a52914d44b1",
                    "servers": [{"rootUrl": good.uri(), "versions": ["1.0.0"]}]
                }])),
        )
        .expect(1)
        .mount(&good)
        .await;
    let client = Client::builder()
        .limits(limits())
        .destination_policy(policy)
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", good.uri())).unwrap())
        .build()
        .unwrap();
    let found = client.discover(&tei).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].product_release.to_string(),
        "d4d9f54a-abcf-11ee-ac79-1a52914d44b1"
    );
    assert_eq!(found[0].api_version, "1.0.0");
}

#[tokio::test]
async fn bearer_401_does_not_fail_over() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    let policy = DestinationPolicy::allow_origin(&Url::parse(&first.uri()).unwrap())
        .unwrap()
        .and_allow(&Url::parse(&second.uri()).unwrap())
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [
                {"url": first.uri(), "versions": ["1.0.0"], "priority": 1},
                {"url": second.uri(), "versions": ["1.0.0"], "priority": 0.2}
            ]
        })))
        .mount(&second)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .expect(1)
        .mount(&first)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("[]", "application/json"))
        .expect(0)
        .mount(&second)
        .await;
    let client = Client::builder()
        .limits(limits())
        .destination_policy(policy)
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", second.uri())).unwrap())
        .build()
        .unwrap();
    let error = client.discover(&tei).await.unwrap_err();
    assert!(matches!(error, ClientError::AuthenticationRequired));
}

#[tokio::test]
async fn bearer_challenge_exchanges_an_api_key_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"tok","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .and(header("authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(product_json(), "application/json"))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    let url = Url::parse(&server.uri()).unwrap();
    let client = Client::builder()
        .limits(limits())
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .api_key(ApiKey::new("key-id", "key-secret"))
        .build()
        .unwrap();
    let product = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap();
    assert_eq!(product.name, "Widget");
}

#[tokio::test]
async fn artifact_checksum_covers_decoded_bytes_and_not_a_redirect_with_credentials() {
    let server = MockServer::start().await;
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, b"tea");
    let digest = hex_encode(&hasher.finalize());
    Mock::given(method("GET"))
        .and(path(
            "/v1.0.0/artifact/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d/1/download",
        ))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/files/sbom", server.uri())),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/sbom"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "W/\"weak\"")
                .set_body_raw("tea", "application/json"),
        )
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/sbom"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(1)
        .mount(&server)
        .await;
    let checksum = Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap();
    let fetch = client_for(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(PRODUCT_ID).unwrap(),
            1,
            Some("application/json"),
            &[checksum],
            None,
        )
        .await
        .unwrap();
    let ArtifactFetch::Body(body) = fetch else {
        panic!("expected a body");
    };
    assert_eq!(body.bytes, b"tea");
    assert!(!body.strong_etag);
    assert!(matches!(
        body.integrity.outcome,
        libtea_core::IntegrityOutcome::Verified
    ));

    let wrong = Checksum::new(
        ChecksumAlgorithm::Sha256,
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    let failed = client_for(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(PRODUCT_ID).unwrap(),
            1,
            None,
            &[wrong],
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(failed, ClientError::Integrity { .. }));
}

#[tokio::test]
async fn oversized_json_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{PRODUCT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(product_json(), "application/json"))
        .mount(&server)
        .await;
    let url = Url::parse(&server.uri()).unwrap();
    let mut tight = limits();
    tight.max_json_bytes = 16;
    let client = Client::builder()
        .limits(tight)
        .destination_policy(DestinationPolicy::allow_origin(&url).unwrap())
        .build()
        .unwrap();
    let error = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(PRODUCT_ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::TooLarge));
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::new();
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}
