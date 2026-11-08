//! A download whose only declared checksum is SHA-1 is rejected unless the
//! client opts in with `ClientBuilder::allow_weak_checksums`.

use std::time::Duration;

use libtea_client::{ArtifactFetch, Client, ClientError, Limits};
use libtea_core::{ArtifactId, Checksum, ChecksumAlgorithm, DestinationPolicy, IntegrityOutcome};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
// SHA-1("tea")
const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";

async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea", "application/json"))
        .mount(&server)
        .await;
    server
}

fn client(server: &MockServer, allow_weak: bool) -> Client {
    Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .allow_weak_checksums(allow_weak)
        .build()
        .unwrap()
}

async fn download(client: &Client, server: &MockServer) -> Result<ArtifactFetch, ClientError> {
    let checksum = Checksum::new(ChecksumAlgorithm::Sha1, SHA1_TEA).unwrap();
    client
        .service(&server.uri(), "1.0.0")
        .unwrap()
        .download_artifact(ArtifactId::parse(ID).unwrap(), 1, None, &[checksum], None)
        .await
}

#[tokio::test]
async fn sha1_only_download_is_an_integrity_error_by_default() {
    let server = server().await;
    let error = download(&client(&server, false), &server)
        .await
        .unwrap_err();
    match error {
        ClientError::Integrity {
            outcome, report, ..
        } => {
            assert_eq!(outcome, IntegrityOutcome::Weak);
            assert!(report.checks.iter().all(|check| check.weak));
        }
        other => panic!("expected a weak integrity error, got {other:?}"),
    }
}

#[tokio::test]
async fn opt_in_accepts_a_sha1_only_match_and_keeps_the_weak_flag() {
    let server = server().await;
    let fetched = download(&client(&server, true), &server).await.unwrap();
    let ArtifactFetch::Body(body) = fetched else {
        panic!("expected a body");
    };
    assert_eq!(body.bytes, b"tea");
    assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
    assert!(body.integrity.checks[0].weak);
}
