//! Protocol behavior that the audit had not executed.
//!
//! These tests use a local mock. They do not contact a public TEA server.

use std::time::Duration;

use libtea_client::{
    ApiKey, ArtifactFetch, CleFetch, Client, ClientError, Limits, SignatureFetch,
};
use libtea_core::{Checksum, ChecksumAlgorithm, DestinationPolicy, ListParams, ProductId, Tei};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{
    header, header_exists, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn limits() -> Limits {
    Limits {
        initial_backoff: Duration::ZERO,
        max_retries: 0,
        request_timeout: Duration::from_secs(5),
        connect_timeout: Duration::from_secs(2),
        ..Limits::default()
    }
}

fn allow(server: &MockServer) -> DestinationPolicy {
    DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap()
}

fn client(server: &MockServer) -> Client {
    Client::builder()
        .limits(limits())
        .destination_policy(allow(server))
        .build()
        .unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn params<S>(page_size: Option<u32>, page_token: Option<String>) -> ListParams<S> {
    ListParams {
        page_size,
        page_token,
        sort_field: None,
        sort_order: None,
        id_type: None,
        id_value: None,
    }
}

fn product_page(has_next: bool, token: Option<&str>, count: usize) -> serde_json::Value {
    let results: Vec<_> = (0..count)
        .map(|_| {
            serde_json::json!({
                "uuid": ID,
                "name": "Widget",
                "identifiers": [{"idType": "PURL", "idValue": "pkg:generic/widget@1"}]
            })
        })
        .collect();
    serde_json::json!({
        "hasNext": has_next,
        "nextPageToken": token,
        "results": results,
    })
}

#[tokio::test]
async fn head_ignores_representation_content_length() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .insert_header("content-length", "128")
                .insert_header("etag", "\"abc\""),
        )
        .expect(1)
        .mount(&server)
        .await;
    let headers = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .head_artifact(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None)
        .await
        .unwrap();
    assert_eq!(
        headers.get("content-length").map(String::as_str),
        Some("128")
    );
    assert_eq!(headers.get("etag").map(String::as_str), Some("\"abc\""));
    assert_eq!(
        headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
}

#[tokio::test]
async fn head_without_content_length_returns_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "application/json"))
        .mount(&server)
        .await;
    let headers = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .head_artifact(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None)
        .await
        .unwrap();
    assert!(!headers.contains_key("content-length"));
    assert_eq!(
        headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
}

#[tokio::test]
async fn head_bearer_challenge_is_an_authentication_error() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .head_artifact(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::AuthenticationRequired));
}

#[tokio::test]
async fn head_redirect_uses_the_final_headers_and_strips_a_real_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"tok","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .and(header("authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/files/meta", server.uri())),
        )
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/files/meta"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/files/meta"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .insert_header("content-length", "4")
                .insert_header("etag", "\"meta\""),
        )
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::builder()
        .limits(limits())
        .destination_policy(allow(&server))
        .api_key(ApiKey::new("id", "secret"))
        .build()
        .unwrap();
    let headers = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .head_artifact(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None)
        .await
        .unwrap();
    assert_eq!(headers.get("etag").map(String::as_str), Some("\"meta\""));
}

#[tokio::test]
async fn bearer_token_is_kept_under_the_prefix_and_stripped_after_that() {
    let server = MockServer::start().await;
    let body = b"tea";
    let digest = sha256(body);
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"tok","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .and(header("authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/v1.0.0/files/inside", server.uri())),
        )
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/files/inside"))
        .and(header("authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/files/outside", server.uri())),
        )
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/files/inside"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/outside"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/outside"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "application/json"))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    let checksum = Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap();
    let client = Client::builder()
        .limits(limits())
        .destination_policy(allow(&server))
        .api_key(ApiKey::new("id", "secret"))
        .build()
        .unwrap();
    let fetched = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[checksum],
            None,
        )
        .await
        .unwrap();
    let ArtifactFetch::Body(downloaded) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(downloaded.bytes, body);
    assert_eq!(
        downloaded.integrity.outcome,
        libtea_core::IntegrityOutcome::Verified
    );
    assert_ne!(downloaded.etag.as_deref(), Some(digest.as_str()));
}

#[tokio::test]
async fn redirect_to_another_origin_is_refused_before_a_credential_is_sent() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/files/sbom", second.uri())),
        )
        .expect(1)
        .mount(&first)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/sbom"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea", "application/octet-stream"))
        .expect(0)
        .mount(&second)
        .await;
    let error = client(&first)
        .service(&first.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[],
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Policy(_)));
}

#[tokio::test]
async fn conditional_get_304_is_not_a_checksum_and_has_no_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .and(header("if-none-match", "\"abc\""))
        .respond_with(
            ResponseTemplate::new(304)
                .insert_header("etag", "\"abc\"")
                .insert_header("cache-control", "public, max-age=60")
                .insert_header("content-length", "100"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let fetched = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[],
            Some("\"abc\""),
        )
        .await
        .unwrap();
    let ArtifactFetch::NotModified {
        etag,
        cache_control,
        ..
    } = fetched
    else {
        panic!("expected 304");
    };
    assert_eq!(etag.as_deref(), Some("\"abc\""));
    assert_eq!(cache_control.as_deref(), Some("public, max-age=60"));
}

#[tokio::test]
async fn zlib_deflate_is_decoded_before_the_checksum() {
    let raw = b"artifact-bytes";
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, raw).unwrap();
    let zlib = encoder.finish().unwrap();
    let digest = sha256(raw);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "deflate")
                .set_body_raw(zlib, "application/octet-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let checksum = Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap();
    let fetched = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[checksum],
            None,
        )
        .await
        .unwrap();
    let ArtifactFetch::Body(body) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(body.bytes, raw);
    assert_eq!(
        body.integrity.outcome,
        libtea_core::IntegrityOutcome::Verified
    );
}

#[tokio::test]
async fn discovery_403_does_not_fail_over_or_fetch_a_token() {
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1").unwrap();
    let policy = allow(&first)
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
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"tok","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(0)
        .mount(&first)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(
            r#"{"error":"INVALID_REQUEST","message":"denied"}"#,
            "application/json",
        ))
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
        .api_key(ApiKey::new("id", "secret"))
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", second.uri())).unwrap())
        .build()
        .unwrap();
    let error = client.discover(&tei).await.unwrap_err();
    assert!(matches!(error, ClientError::Forbidden));
}

#[tokio::test]
async fn product_pages_keep_tokens_opaque_and_do_not_follow_urls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageSize", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(
            true,
            Some("https://169.254.169.254/latest"),
            2,
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageToken", "https://169.254.169.254/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(false, None, 1)))
        .with_priority(1)
        .expect(2)
        .mount(&server)
        .await;
    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let mut query = params(Some(2), None);
    let first = service.query_products(&query).await.unwrap();
    assert!(first.has_next);
    assert_eq!(first.results.len(), 2);
    assert_eq!(
        first.next_page_token.as_deref(),
        Some("https://169.254.169.254/latest")
    );
    query.page_token = first.next_page_token;
    let last = service.query_products(&query).await.unwrap();
    assert!(!last.has_next);
    assert_eq!(last.results.len(), 1);
    let again = service.query_products(&query).await.unwrap();
    assert_eq!(again.results.len(), 1);
}

#[tokio::test]
async fn empty_page_and_malformed_page_and_oversized_page_size() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(false, None, 0)))
        .expect(1)
        .mount(&server)
        .await;
    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let empty = service.query_products(&params(None, None)).await.unwrap();
    assert!(empty.results.is_empty());
    assert!(!empty.has_next);

    let huge = params(Some(101), None);
    assert!(service.query_products(&huge).await.is_err());

    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"results":[]}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let error = service
        .query_components(&params(None, None))
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Json(_)));
}

#[tokio::test]
async fn not_acceptable_is_an_api_error_and_a_missing_json_type_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(ResponseTemplate::new(406).set_body_raw(
            r#"{"error":"NO_ACCEPTABLE_FORMAT","message":"no format"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap_err();
    match error {
        ClientError::Api {
            status: 406,
            tea_error: Some(code),
            ..
        } => assert_eq!(code, "NO_ACCEPTABLE_FORMAT"),
        other => panic!("unexpected {other}"),
    }

    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/component/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .expect(1)
        .mount(&server)
        .await;
    let error = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_component(libtea_core::ComponentId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Model(_)));
}

#[tokio::test]
async fn signature_absence_is_not_verification_and_unknown_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/signature/download")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"SIGNATURE_NOT_FOUND"}"#, "application/json"),
        )
        .with_priority(1)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let missing = service
        .download_signature(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None, None)
        .await
        .unwrap();
    assert!(matches!(missing, SignatureFetch::NotPublished));

    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/signature/download")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    let error = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_signature(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None, None)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Api {
            status: 404,
            tea_error: Some(ref code),
            ..
        } if code == "OBJECT_UNKNOWN"
    ));
}

#[tokio::test]
async fn signature_bytes_are_not_reported_as_verified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/signature/download")))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw("sig-bytes", "application/pgp-signature"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let fetched = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_signature(libtea_core::ArtifactId::parse(ID).unwrap(), 1, None, None)
        .await
        .unwrap();
    let SignatureFetch::Present(body) = fetched else {
        panic!("expected signature bytes");
    };
    assert_eq!(body.bytes, b"sig-bytes");
    assert_eq!(
        body.integrity.outcome,
        libtea_core::IntegrityOutcome::NotDeclared
    );
}

#[tokio::test]
async fn cle_not_implemented_is_distinct_from_an_unknown_object() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}/cle")))
        .respond_with(ResponseTemplate::new(404).set_body_raw(
            r#"{"error":"NOT_IMPLEMENTED","message":"no cle"}"#,
            "application/json",
        ))
        .with_priority(1)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let cle = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .get_product_cle(ProductId::parse(ID).unwrap())
        .await
        .unwrap();
    assert!(matches!(cle, CleFetch::NotImplemented { .. }));
}

#[tokio::test]
async fn redirect_limit_and_gzip_expansion_limit_are_enforced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "location",
            format!("{}/v1.0.0/artifact/{ID}/1/download", server.uri()),
        ))
        .mount(&server)
        .await;
    let mut tight = limits();
    tight.max_redirects = 1;
    let client = Client::builder()
        .limits(tight)
        .destination_policy(allow(&server))
        .build()
        .unwrap();
    let error = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[],
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::TooManyRedirects));

    let raw = vec![b'a'; 4096];
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &raw).unwrap();
    let gzip = encoder.finish().unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_raw(gzip, "application/octet-stream"),
        )
        .mount(&server)
        .await;
    let mut tight = limits();
    tight.max_artifact_bytes = 64;
    let client = Client::builder()
        .limits(tight)
        .destination_policy(allow(&server))
        .build()
        .unwrap();
    let error = client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(
            libtea_core::ArtifactId::parse(ID).unwrap(),
            1,
            None,
            &[],
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Model(_) | ClientError::TooLarge
    ));
}

fn component_page(has_next: bool, token: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "hasNext": has_next,
        "nextPageToken": token,
        "results": [{
            "uuid": ID,
            "name": "Part",
            "identifiers": []
        }]
    })
}

#[tokio::test]
async fn type_only_identifier_filter_is_serialized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .and(query_param("idType", "COMPLIANCE_DOCUMENT"))
        .and(query_param_is_missing("idValue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(component_page(false, None)))
        .expect(1)
        .mount(&server)
        .await;
    let mut query = params(None, None);
    query.id_type = Some(libtea_core::enums::IdentifierType::ComplianceDocument);
    let page = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_components(&query)
        .await
        .unwrap();
    assert_eq!(page.results.len(), 1);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let pairs: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0, "idType");
    assert_eq!(pairs[0].1, "COMPLIANCE_DOCUMENT");
}

#[tokio::test]
async fn both_identifier_filters_are_encoded_in_the_query() {
    let server = MockServer::start().await;
    let value = "pkg:generic/widget@1 a&b";
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("idType", "PURL"))
        .and(query_param("idValue", value))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(false, None, 1)))
        .expect(1)
        .mount(&server)
        .await;
    let mut query = params(None, None);
    query.id_type = Some(libtea_core::enums::IdentifierType::Purl);
    query.id_value = Some(value.to_owned());
    let page = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_products(&query)
        .await
        .unwrap();
    assert_eq!(page.results.len(), 1);
    let requests = server.received_requests().await.unwrap();
    let raw = requests[0].url.query().unwrap();
    assert!(raw.contains("idValue="));
    assert!(raw.contains("%26"));
    assert!(!raw.contains("&b"));
    let pairs: Vec<_> = requests[0].url.query_pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0, "idType");
    assert_eq!(pairs[1].0, "idValue");
    assert_eq!(pairs[1].1, value);
}

#[tokio::test]
async fn absent_identifier_filters_are_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageSize", "25"))
        .and(query_param_is_missing("idType"))
        .and(query_param_is_missing("idValue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(false, None, 0)))
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_products(&params(Some(25), None))
        .await
        .unwrap();
    assert!(page.results.is_empty());
    let raw = server.received_requests().await.unwrap()[0]
        .url
        .query()
        .unwrap()
        .to_owned();
    assert_eq!(raw, "pageSize=25");
}

#[tokio::test]
async fn default_list_params_send_no_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": false,
            "results": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let query = ListParams::<libtea_core::enums::ComponentSort>::default();
    let page = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_components(&query)
        .await
        .unwrap();
    assert!(page.results.is_empty());
    assert!(!page.has_next);
    assert!(server.received_requests().await.unwrap()[0]
        .url
        .query()
        .is_none());
}

#[tokio::test]
async fn value_only_identifier_filter_sends_no_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let mut query = params(None, None);
    query.id_value = Some("pkg:generic/widget@1".to_owned());
    let error = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_products(&query)
        .await
        .unwrap_err();
    match error {
        ClientError::Model(libtea_core::ModelError::Invalid(message)) => {
            assert_eq!(message, "idValue requires idType");
        }
        other => panic!("unexpected error: {other}"),
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn type_only_filter_stays_on_the_next_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .and(query_param("idType", "COMPLIANCE_DOCUMENT"))
        .and(query_param("pageSize", "1"))
        .and(query_param_is_missing("idValue"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(component_page(true, Some("page-2"))),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .and(query_param("idType", "COMPLIANCE_DOCUMENT"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "page-2"))
        .and(query_param_is_missing("idValue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(component_page(false, None)))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let mut query = params(Some(1), None);
    query.id_type = Some(libtea_core::enums::IdentifierType::ComplianceDocument);
    let first = service.query_components(&query).await.unwrap();
    assert!(first.has_next);
    assert_eq!(first.next_page_token.as_deref(), Some("page-2"));
    query.page_token = first.next_page_token;
    let last = service.query_components(&query).await.unwrap();
    assert!(!last.has_next);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let pairs: Vec<_> = request.url.query_pairs().collect();
        assert!(pairs
            .iter()
            .any(|(key, value)| key == "idType" && value == "COMPLIANCE_DOCUMENT"));
        assert!(pairs.iter().all(|(key, _)| key != "idValue"));
    }
    let second: Vec<_> = requests[1].url.query_pairs().collect();
    assert!(second
        .iter()
        .any(|(key, value)| key == "pageToken" && value == "page-2"));
    assert!(second
        .iter()
        .any(|(key, value)| key == "pageSize" && value == "1"));
}

#[tokio::test]
async fn list_queries_keep_unrelated_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageSize", "10"))
        .and(query_param("sortField", "name"))
        .and(query_param("sortOrder", "desc"))
        .and(query_param_is_missing("idType"))
        .and(query_param_is_missing("idValue"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_page(false, None, 1)))
        .expect(1)
        .mount(&server)
        .await;
    let mut query = params(Some(10), None);
    query.sort_field = Some(libtea_core::enums::ProductSort::Name);
    query.sort_order = Some(libtea_core::enums::SortOrder::Desc);
    let page = client(&server)
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .query_products(&query)
        .await
        .unwrap();
    assert_eq!(page.results.len(), 1);
}
