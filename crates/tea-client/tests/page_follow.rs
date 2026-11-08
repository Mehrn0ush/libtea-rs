//! Wiremock coverage for opt-in [`libtea_client::follow_pages`].

use std::time::Duration;

use futures_util::StreamExt;
use libtea_client::{follow_pages, Client, Limits, PageFollowLimits, PageFollowStop};
use libtea_core::enums::ProductSort;
use libtea_core::{DestinationPolicy, ListParams};
use url::Url;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
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

fn page(has_next: bool, token: Option<&str>, names: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "hasNext": has_next,
        "nextPageToken": token,
        "results": names.iter().map(|name| serde_json::json!({
            "uuid": ID,
            "name": name,
            "identifiers": []
        })).collect::<Vec<_>>()
    })
}

#[tokio::test]
async fn follow_pages_multi_page_then_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageSize", "1"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(true, Some("t2"), &["A"])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .and(query_param("pageToken", "t2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(false, None, &["B"])))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let stream = follow_pages(
        ListParams::<ProductSort>::default().with_page_size(1),
        PageFollowLimits::default()
            .with_max_pages(10)
            .with_max_items(100),
        move |params| {
            let service = service.clone();
            Box::pin(async move { service.query_products(&params).await })
        },
    );
    futures_util::pin_mut!(stream);
    let first = stream.next().await.unwrap().unwrap();
    assert!(first.stop.is_none());
    assert_eq!(first.page.results[0].name, "A");
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(second.stop, Some(PageFollowStop::Exhausted));
    assert_eq!(second.page.results[0].name, "B");
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn follow_pages_empty_page_exhausts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(false, None, &[])))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let stream = follow_pages(
        ListParams::<ProductSort>::default(),
        PageFollowLimits::default(),
        move |params| {
            let service = service.clone();
            Box::pin(async move { service.query_products(&params).await })
        },
    );
    futures_util::pin_mut!(stream);
    let only = stream.next().await.unwrap().unwrap();
    assert!(only.page.results.is_empty());
    assert_eq!(only.stop, Some(PageFollowStop::Exhausted));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn follow_pages_hits_max_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(
            true,
            Some("more"),
            &["X", "Y"],
        )))
        .expect(1)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let stream = follow_pages(
        ListParams::<ProductSort>::default(),
        PageFollowLimits::default()
            .with_max_pages(10)
            .with_max_items(2),
        move |params| {
            let service = service.clone();
            Box::pin(async move { service.query_products(&params).await })
        },
    );
    futures_util::pin_mut!(stream);
    let only = stream.next().await.unwrap().unwrap();
    assert_eq!(only.stop, Some(PageFollowStop::MaxItems));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn follow_pages_repeated_token_guard() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/products"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(true, Some("loop"), &["Z"])))
        .expect(2)
        .mount(&server)
        .await;

    let service = client(&server).service(&server.uri(), "1.0.0").unwrap();
    let stream = follow_pages(
        ListParams::<ProductSort>::default(),
        PageFollowLimits::default()
            .with_max_pages(10)
            .with_max_items(100),
        move |params| {
            let service = service.clone();
            Box::pin(async move { service.query_products(&params).await })
        },
    );
    futures_util::pin_mut!(stream);
    assert!(stream.next().await.unwrap().unwrap().stop.is_none());
    let last = stream.next().await.unwrap().unwrap();
    assert_eq!(last.stop, Some(PageFollowStop::RepeatedToken));
    assert!(stream.next().await.is_none());
}
