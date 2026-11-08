//! `429` and `503` handling: `Retry-After` is honored up to a cap, then the
//! error carries what the server asked for.

use std::time::{Duration, Instant};

use libtea_client::{Client, ClientError, FileDownloadOptions, Limits, Service};
use libtea_core::{ArtifactId, DestinationPolicy, ProductReleaseId};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ART: &str = "11111111-1111-4111-8111-111111111111";

fn release_body() -> serde_json::Value {
    serde_json::json!({
        "uuid": REL, "product": REL, "version": "1.0.0",
        "createdDate": "2024-01-01T00:00:00Z", "components": [],
        "latestCollection": {"uuid": REL, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"}, "artifacts": []}
    })
}

fn service(server: &MockServer, retries: u32, cap: Duration) -> Service {
    Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(retries)
                .with_max_retry_after(cap),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .build()
        .unwrap()
        .service(&server.uri(), "1.0.0")
        .unwrap()
}

async fn fetch(service: &Service) -> Result<(), ClientError> {
    service
        .get_product_release(ProductReleaseId::parse(REL).unwrap())
        .await
        .map(|_| ())
}

async fn count(server: &MockServer) -> usize {
    server.received_requests().await.unwrap().len()
}

/// A mock that answers `first` once and `200` afterwards.
async fn then_ok(server: &MockServer, p: &str, first: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(p))
        .respond_with(first)
        .up_to_n_times(1)
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(p))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_body()))
        .with_priority(5)
        .mount(server)
        .await;
}

fn release_path() -> String {
    format!("/v1.0.0/productRelease/{REL}")
}

#[tokio::test]
async fn a_429_with_retry_after_is_waited_out_and_retried() {
    let server = MockServer::start().await;
    then_ok(
        &server,
        &release_path(),
        ResponseTemplate::new(429).insert_header("retry-after", "1"),
    )
    .await;
    let started = Instant::now();
    fetch(&service(&server, 2, Duration::from_secs(5)))
        .await
        .unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(900),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(count(&server).await, 2);
}

#[tokio::test]
async fn a_503_with_retry_after_zero_retries_at_once() {
    let server = MockServer::start().await;
    then_ok(
        &server,
        &release_path(),
        ResponseTemplate::new(503).insert_header("retry-after", "0"),
    )
    .await;
    let started = Instant::now();
    fetch(&service(&server, 2, Duration::from_secs(5)))
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_millis(900));
    assert_eq!(count(&server).await, 2);
}

#[tokio::test]
async fn a_date_in_the_past_means_no_wait() {
    let server = MockServer::start().await;
    then_ok(
        &server,
        &release_path(),
        ResponseTemplate::new(429).insert_header("retry-after", "Sun, 06 Nov 1994 08:49:37 GMT"),
    )
    .await;
    let started = Instant::now();
    fetch(&service(&server, 2, Duration::from_secs(5)))
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_millis(900));
    assert_eq!(count(&server).await, 2);
}

#[tokio::test]
async fn a_wait_longer_than_the_cap_fails_at_once_and_reports_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "3600"))
        .mount(&server)
        .await;
    let started = Instant::now();
    let error = fetch(&service(&server, 5, Duration::from_secs(2)))
        .await
        .unwrap_err();
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "must not sleep for an hour"
    );
    match error {
        ClientError::Throttled {
            status,
            retry_after_secs,
            ..
        } => {
            assert_eq!(status, 429);
            assert_eq!(retry_after_secs, Some(3600));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        count(&server).await,
        1,
        "no retry when the wait is over the cap"
    );
}

#[tokio::test]
async fn without_retry_after_the_client_backs_off_then_gives_up() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;
    let error = fetch(&service(&server, 3, Duration::from_secs(5)))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Throttled {
            status: 429,
            retry_after_secs: None,
            ..
        }
    ));
    assert_eq!(count(&server).await, 4, "one try plus max_retries");
    assert!(error.to_string().contains("HTTP 429"));
}

#[tokio::test]
async fn garbage_retry_after_is_ignored() {
    let server = MockServer::start().await;
    for junk in [
        "soon",
        "-1",
        "1.5",
        "5 minutes",
        "999999999999999999999999 x",
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", junk))
            .mount(&server)
            .await;
        let error = fetch(&service(&server, 1, Duration::from_secs(5)))
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                ClientError::Throttled {
                    status: 503,
                    retry_after_secs: None,
                    ..
                }
            ),
            "{junk}: {error:?}"
        );
        assert_eq!(count(&server).await, 2, "{junk}");
    }
}

#[tokio::test]
async fn a_huge_retry_after_saturates_and_is_over_the_cap() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("retry-after", "99999999999999999999"),
        )
        .mount(&server)
        .await;
    let error = fetch(&service(&server, 2, Duration::from_secs(5)))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Throttled {
            retry_after_secs: Some(u64::MAX),
            ..
        }
    ));
    assert_eq!(count(&server).await, 1);
}

#[tokio::test]
async fn the_streaming_download_honors_retry_after_too() {
    let server = MockServer::start().await;
    let p = format!("/v1.0.0/artifact/{ART}/1/download");
    Mock::given(method("GET"))
        .and(path(p.clone()))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(p))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(b"data".to_vec(), "application/octet-stream"),
        )
        .with_priority(5)
        .mount(&server)
        .await;
    let out = std::env::temp_dir().join(format!("libtea-throttle-{}.bin", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let started = Instant::now();
    service(&server, 2, Duration::from_secs(5))
        .download_artifact_to_file(
            ArtifactId::parse(ART).unwrap(),
            1,
            None,
            &[],
            &out,
            &FileDownloadOptions::new(),
        )
        .await
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert_eq!(std::fs::read(&out).unwrap(), b"data");
    let _ = std::fs::remove_file(&out);
}

#[tokio::test]
async fn a_429_on_the_token_exchange_is_still_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(release_path()))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .mount(&server)
        .await;
    let svc = Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(3),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .api_key(libtea_client::ApiKey::new("id", "secret"))
        .build()
        .unwrap()
        .service(&server.uri(), "1.0.0")
        .unwrap();
    assert!(fetch(&svc).await.is_err());
    let posts = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.method.as_str() == "POST")
        .count();
    assert_eq!(posts, 1);
}
