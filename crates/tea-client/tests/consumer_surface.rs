//! Mock coverage for Consumer operations that lacked dedicated wiremock fixtures.
//!
//! These tests do not contact a public TEA server.

use std::time::Duration;

use libtea_client::{CleFetch, Client, ClientError, Limits};
use libtea_core::{
    enums::{CollectionSort, IdentifierType, ReleaseSort, SortOrder},
    ArtifactId, ComponentId, ComponentReleaseId, DestinationPolicy, ListParams, ProductId,
    ProductReleaseId,
};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ID2: &str = "3aa4bf8d-9fbc-5f1c-ac9c-2b1d5d5d2d3e";

fn limits() -> Limits {
    Limits::default()
        .with_initial_backoff(Duration::ZERO)
        .with_max_retries(0)
        .with_request_timeout(Duration::from_secs(5))
        .with_connect_timeout(Duration::from_secs(2))
}

fn client(server: &MockServer) -> Client {
    Client::builder()
        .limits(limits())
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .build()
        .unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn empty_collection(version: u64) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "version": version,
        "createdDate": "2024-01-01T00:00:00Z",
        "belongsTo": "PRODUCT_RELEASE",
        "updateReason": {"type": "INITIAL_RELEASE"},
        "artifacts": []
    })
}

fn product_release_body(version: &str) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "product": ID2,
        "version": version,
        "createdDate": "2024-01-01T00:00:00Z",
        "components": [],
        "latestCollection": empty_collection(1)
    })
}

fn component_release_body(version: &str) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "component": ID2,
        "version": version,
        "createdDate": "2024-01-01T00:00:00Z",
        "latestCollection": {
            "uuid": ID,
            "version": 1,
            "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "COMPONENT_RELEASE",
            "updateReason": {"type": "INITIAL_RELEASE"},
            "artifacts": []
        }
    })
}

fn release_page(versions: &[&str], has_next: bool, token: Option<&str>) -> serde_json::Value {
    let results: Vec<_> = versions
        .iter()
        .map(|version| {
            serde_json::json!({
                "uuid": ID,
                "product": ID2,
                "version": version,
                "createdDate": "2024-01-01T00:00:00Z",
                "components": []
            })
        })
        .collect();
    serde_json::json!({
        "hasNext": has_next,
        "nextPageToken": token,
        "results": results,
    })
}

fn collection_page(versions: &[u64], has_next: bool, token: Option<&str>) -> serde_json::Value {
    let results: Vec<_> = versions.iter().copied().map(empty_collection).collect();
    serde_json::json!({
        "hasNext": has_next,
        "nextPageToken": token,
        "results": results,
    })
}

fn artifact_meta(version: u64, media: &str, digest: &str) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "version": version,
        "type": "BOM",
        "createdDate": "2024-01-01T00:00:00Z",
        "formats": [{
            "mediaType": media,
            "checksums": [{"algType": "SHA-256", "algValue": digest}]
        }]
    })
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
async fn list_and_query_product_releases_paths_and_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}/releases")))
        .and(query_param("pageSize", "2"))
        .and(query_param("sortField", "version"))
        .and(query_param("sortOrder", "asc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_page(
            &["1.0.0", "1.1.0"],
            false,
            None,
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/productReleases"))
        .and(query_param("idType", "PURL"))
        .and(query_param("idValue", "pkg:generic/widget@1"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_page(
            &["1.0.0"],
            false,
            None,
        )))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let listed = service
        .list_product_releases(
            ProductId::parse(ID).unwrap(),
            &ListParams::default()
                .with_page_size(2)
                .with_sort_field(ReleaseSort::Version)
                .with_sort_order(SortOrder::Asc),
        )
        .await
        .unwrap();
    assert_eq!(listed.results.len(), 2);
    assert!(!listed.has_next);

    let queried = service
        .query_product_releases(
            &ListParams::default()
                .with_id_type(IdentifierType::Purl)
                .with_id_value("pkg:generic/widget@1"),
        )
        .await
        .unwrap();
    assert_eq!(queried.results[0].version, "1.0.0");
}

#[tokio::test]
async fn list_and_query_component_releases_paths() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/component/{ID}/releases")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": false,
            "results": [{
                "uuid": ID,
                "component": ID2,
                "version": "1.0.0",
                "createdDate": "2024-01-01T00:00:00Z"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/componentReleases"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "tok/with spaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": false,
            "results": [{
                "uuid": ID2,
                "component": ID2,
                "version": "1.1.0",
                "createdDate": "2024-01-02T00:00:00Z"
            }]
        })))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/componentReleases"))
        .and(query_param("pageSize", "1"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": true,
            "nextPageToken": "tok/with spaces",
            "results": [{
                "uuid": ID,
                "component": ID2,
                "version": "1.0.0",
                "createdDate": "2024-01-01T00:00:00Z"
            }]
        })))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let listed = service
        .list_component_releases(ComponentId::parse(ID).unwrap(), &ListParams::default())
        .await
        .unwrap();
    assert_eq!(listed.results.len(), 1);

    let mut query = ListParams::default().with_page_size(1);
    let first = service.query_component_releases(&query).await.unwrap();
    assert!(first.has_next);
    assert_eq!(first.next_page_token.as_deref(), Some("tok/with spaces"));
    query.page_token = first.next_page_token;
    let second = service.query_component_releases(&query).await.unwrap();
    assert_eq!(second.results[0].version, "1.1.0");
}

#[tokio::test]
async fn component_release_collections_and_versioned_gets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/componentRelease/{ID}/collections")))
        .and(query_param("pageSize", "1"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection_page(
            &[1],
            true,
            Some("c-next"),
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/componentRelease/{ID}/collections")))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "c-next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection_page(&[2], false, None)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/componentRelease/{ID}/collection/latest"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_collection(2)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/componentRelease/{ID}/collection/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_collection(1)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/componentRelease/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(component_release_body("1.0.0")))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let id = ComponentReleaseId::parse(ID).unwrap();
    let mut query = ListParams::default().with_page_size(1);
    let first = service
        .list_component_release_collections(id, &query)
        .await
        .unwrap();
    assert_eq!(first.results[0].version, 1);
    query.page_token = first.next_page_token;
    let second = service
        .list_component_release_collections(id, &query)
        .await
        .unwrap();
    assert_eq!(second.results[0].version, 2);
    assert_eq!(
        service
            .get_latest_component_release_collection(id)
            .await
            .unwrap()
            .version,
        2
    );
    assert_eq!(
        service
            .get_component_release_collection(id, 1)
            .await
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        service
            .get_component_release(id)
            .await
            .unwrap()
            .release
            .version,
        "1.0.0"
    );
}

#[tokio::test]
async fn cle_success_and_object_unknown() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}/cle")))
        .respond_with(ResponseTemplate::new(200).set_body_json(cle_body()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/cle")))
        .respond_with(ResponseTemplate::new(404).set_body_raw(
            r#"{"error":"OBJECT_UNKNOWN","message":"missing"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/component/{ID}/cle")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"NOT_IMPLEMENTED"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    match service
        .get_product_cle(ProductId::parse(ID).unwrap())
        .await
        .unwrap()
    {
        CleFetch::Events(cle) => assert_eq!(cle.events.len(), 1),
        other => panic!("expected events, got {other:?}"),
    }
    let unknown = service
        .get_product_release_cle(ProductReleaseId::parse(ID).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(
        unknown,
        ClientError::Api {
            status: 404,
            tea_error: Some(ref code),
            ..
        } if code == "OBJECT_UNKNOWN"
    ));
    assert!(matches!(
        service
            .get_component_cle(ComponentId::parse(ID).unwrap())
            .await
            .unwrap(),
        CleFetch::NotImplemented { .. }
    ));
}

#[tokio::test]
async fn artifact_latest_versus_revision_and_media_type() {
    let body = b"tea-artifact";
    let digest = sha256(body);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/latest")))
        .respond_with(ResponseTemplate::new(200).set_body_json(artifact_meta(
            2,
            "application/vnd.cyclonedx+json",
            &digest,
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(artifact_meta(
            1,
            "application/spdx+json",
            &digest,
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/2/download")))
        .and(query_param("mediaType", "application/vnd.cyclonedx+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"strong-1\"")
                .set_body_raw(body.as_slice(), "application/vnd.cyclonedx+json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let id = ArtifactId::parse(ID).unwrap();
    let latest = service.get_latest_artifact(id).await.unwrap();
    assert_eq!(latest.version, 2);
    let v1 = service.get_artifact(id, 1).await.unwrap();
    assert_eq!(v1.formats[0].media_type, "application/spdx+json");
    let fetched = service
        .download_format(&latest, "application/vnd.cyclonedx+json", None)
        .await
        .unwrap();
    let libtea_client::ArtifactFetch::Body(downloaded) = fetched else {
        panic!("expected body");
    };
    assert_eq!(downloaded.bytes, body);
    assert_eq!(
        downloaded.integrity.outcome,
        libtea_core::IntegrityOutcome::Verified
    );
}

#[tokio::test]
async fn pagination_anomalies_are_visible_to_callers() {
    let server = MockServer::start().await;
    // Empty intermediate page still carries a continuation token.
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collections")))
        .and(query_param("pageSize", "2"))
        .and(query_param("sortField", "version"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection_page(
            &[],
            true,
            Some("mid-empty"),
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collections")))
        .and(query_param("pageSize", "2"))
        .and(query_param("sortField", "version"))
        .and(query_param("pageToken", "mid-empty"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection_page(
            &[3, 4],
            false,
            None,
        )))
        .expect(1)
        .mount(&server)
        .await;
    // Repeated token: server keeps returning the same page; SDK does not loop.
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageToken", "stuck"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": true,
            "nextPageToken": "stuck",
            "results": [{
                "uuid": ID,
                "name": "Widget",
                "identifiers": [{"idType": "PURL", "idValue": "pkg:generic/widget@1"}]
            }]
        })))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageToken", "bad-token"))
        .respond_with(ResponseTemplate::new(400).set_body_raw(
            r#"{"error":"INVALID_PAGE_TOKEN","message":"nope"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/components"))
        .and(query_param("pageToken", "boom"))
        .respond_with(ResponseTemplate::new(500).set_body_raw("fail", "text/plain"))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let rid = ProductReleaseId::parse(ID).unwrap();
    let mut query = ListParams::default()
        .with_page_size(2)
        .with_sort_field(CollectionSort::Version);
    let empty_mid = service
        .list_product_release_collections(rid, &query)
        .await
        .unwrap();
    assert!(empty_mid.results.is_empty());
    assert!(empty_mid.has_next);
    query.page_token = empty_mid.next_page_token;
    let filled = service
        .list_product_release_collections(rid, &query)
        .await
        .unwrap();
    assert_eq!(filled.results.len(), 2);

    let stuck = ListParams::default().with_page_token("stuck");
    let a = service.query_products(&stuck).await.unwrap();
    let b = service.query_products(&stuck).await.unwrap();
    assert_eq!(a.next_page_token, b.next_page_token);
    assert!(a.has_next);

    let bad = service
        .query_products(&ListParams::default().with_page_token("bad-token"))
        .await
        .unwrap_err();
    assert!(matches!(
        bad,
        ClientError::Api {
            status: 400,
            tea_error: Some(ref code),
            ..
        } if code == "INVALID_PAGE_TOKEN"
    ));

    let boom = service
        .query_components(&ListParams::default().with_page_token("boom"))
        .await
        .unwrap_err();
    assert!(matches!(boom, ClientError::Api { status: 500, .. }));
}

#[tokio::test]
async fn product_release_get_and_unknown_identifier() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_release_body("2.0.0")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID2}")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let release = service
        .get_product_release(ProductReleaseId::parse(ID).unwrap())
        .await
        .unwrap();
    assert_eq!(release.release.version, "2.0.0");
    let missing = service
        .get_product(ProductId::parse(ID2).unwrap())
        .await
        .unwrap_err();
    assert!(matches!(
        missing,
        ClientError::Api {
            status: 404,
            tea_error: Some(ref code),
            ..
        } if code == "OBJECT_UNKNOWN"
    ));
}
