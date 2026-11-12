//! Independent wiremock coverage for paths previously listed as Implemented
//! without a dedicated mock fixture (product-release collections by latest /
//! version, and CLE success for every CLE endpoint).

use std::time::Duration;

use libtea_client::{CleFetch, Client, Limits};
use libtea_core::{
    ComponentId, ComponentReleaseId, DestinationPolicy, ProductId, ProductReleaseId,
};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn client(server: &MockServer) -> Client {
    Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0)
                .with_request_timeout(Duration::from_secs(5))
                .with_connect_timeout(Duration::from_secs(2)),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .build()
        .unwrap()
}

fn collection(version: u64) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "version": version,
        "createdDate": "2024-01-01T00:00:00Z",
        "belongsTo": "PRODUCT_RELEASE",
        "updateReason": {"type": "INITIAL_RELEASE"},
        "artifacts": []
    })
}

fn component_collection(version: u64) -> serde_json::Value {
    let mut body = collection(version);
    body["belongsTo"] = serde_json::json!("COMPONENT_RELEASE");
    body
}

fn cle_body() -> serde_json::Value {
    serde_json::json!({
        "events": [{
            "id": 1,
            "type": "released",
            "effective": "2024-01-01T00:00:00Z",
            "published": "2024-01-01T00:00:00Z",
            "version": "1.0.0"
        }],
        "definitions": {"support": []}
    })
}

#[tokio::test]
async fn get_component_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/component/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "name": "Part",
            "identifiers": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let component = service
        .get_component(ComponentId::parse(ID).unwrap())
        .await
        .unwrap();
    assert_eq!(component.name, "Part");
}

#[tokio::test]
async fn product_release_collection_latest_and_by_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/productRelease/{ID}/collection/latest"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection(3)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collection/2")))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection(2)))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let id = ProductReleaseId::parse(ID).unwrap();
    let latest = service
        .get_latest_product_release_collection(id)
        .await
        .unwrap();
    assert_eq!(latest.version, 3);
    let historical = service.get_product_release_collection(id, 2).await.unwrap();
    assert_eq!(historical.version, 2);
}

#[tokio::test]
async fn component_release_collection_latest_and_by_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/componentRelease/{ID}/collection/latest"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(component_collection(4)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/componentRelease/{ID}/collection/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(component_collection(1)))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let id = ComponentReleaseId::parse(ID).unwrap();
    let latest = service
        .get_latest_component_release_collection(id)
        .await
        .unwrap();
    assert_eq!(latest.version, 4);
    let historical = service
        .get_component_release_collection(id, 1)
        .await
        .unwrap();
    assert_eq!(historical.version, 1);
}

#[tokio::test]
async fn download_latest_artifact_and_signature_aliases() {
    use libtea_client::{ArtifactFetch, SignatureFetch};
    use libtea_core::{
        verify_checksums, ArtifactId, Checksum, ChecksumAlgorithm, IntegrityOutcome,
    };
    use sha2::{Digest, Sha256};

    let server = MockServer::start().await;
    let body = b"latest-bytes";
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, body);
    // digest 0.11 output is a byte array without `LowerHex`.
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/latest/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/artifact/{ID}/latest/signature/download"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b"sig", "application/octet-stream"))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let id = ArtifactId::parse(ID).unwrap();
    let checksum = Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap();
    let fetch = service
        .download_latest_artifact(id, Some("application/json"), &[checksum], None)
        .await
        .unwrap();
    let ArtifactFetch::Body(downloaded) = fetch else {
        panic!("expected body");
    };
    assert_eq!(downloaded.bytes.as_slice(), body);
    assert_eq!(downloaded.integrity.outcome, IntegrityOutcome::Verified);
    // Sanity: same digest verifier path as callers use.
    assert_eq!(
        verify_checksums(
            body,
            &[Checksum::new(ChecksumAlgorithm::Sha256, &digest).unwrap()]
        )
        .outcome,
        IntegrityOutcome::Verified
    );

    match service
        .download_latest_signature(id, None, None)
        .await
        .unwrap()
    {
        SignatureFetch::Present(downloaded) => assert_eq!(downloaded.bytes.as_slice(), b"sig"),
        other => panic!("expected signature bytes, got {other:?}"),
    }
}

#[tokio::test]
async fn cle_success_for_every_cle_endpoint() {
    let server = MockServer::start().await;
    for suffix in [
        format!("/v1.0.0/product/{ID}/cle"),
        format!("/v1.0.0/productRelease/{ID}/cle"),
        format!("/v1.0.0/component/{ID}/cle"),
        format!("/v1.0.0/componentRelease/{ID}/cle"),
    ] {
        Mock::given(method("GET"))
            .and(path(suffix))
            .respond_with(ResponseTemplate::new(200).set_body_json(cle_body()))
            .expect(1)
            .mount(&server)
            .await;
    }

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    for result in [
        service.get_product_cle(ProductId::parse(ID).unwrap()).await,
        service
            .get_product_release_cle(ProductReleaseId::parse(ID).unwrap())
            .await,
        service
            .get_component_cle(ComponentId::parse(ID).unwrap())
            .await,
        service
            .get_component_release_cle(ComponentReleaseId::parse(ID).unwrap())
            .await,
    ] {
        match result.unwrap() {
            CleFetch::Events(cle) => assert_eq!(cle.events.len(), 1),
            other => panic!("expected CLE events, got {other:?}"),
        }
    }
}
