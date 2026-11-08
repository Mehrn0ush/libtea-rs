//! `ClientBuilder::lenient_versions`: off by default, on accepts short and
//! same-major versions.

use std::time::Duration;

use libtea_client::{Client, ClientError, Limits};
use libtea_core::{DestinationPolicy, Tei};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "d4d9f54a-abcf-11ee-ac79-1a52914d44b1";
const TEI: &str = "tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1";

/// A server that advertises `version` and answers `/v{version}/discovery`.
async fn server(version: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": [version]}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v{version}/discovery")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "productReleaseUuid": ID,
                "servers": [{"rootUrl": server.uri(), "versions": [version]}]
            }])),
        )
        .mount(&server)
        .await;
    server
}

fn client(server: &MockServer, lenient: Option<bool>) -> Client {
    let mut builder = Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&server.uri()).unwrap()).unwrap(),
        )
        .well_known_fetch_url(Url::parse(&format!("{}/.well-known/tea", server.uri())).unwrap());
    if let Some(lenient) = lenient {
        builder = builder.lenient_versions(lenient);
    }
    builder.build().unwrap()
}

async fn discover(client: &Client) -> Result<Vec<libtea_client::ResolvedRelease>, ClientError> {
    client.discover(&Tei::parse(TEI).unwrap()).await
}

#[tokio::test]
async fn cybeats_shaped_short_version_fails_by_default_and_works_when_lenient() {
    let server = server("1").await;
    for flag in [None, Some(false)] {
        let error = discover(&client(&server, flag)).await.unwrap_err();
        assert!(
            matches!(error, ClientError::Model(_)),
            "{flag:?}: {error:?}"
        );
        assert!(error.to_string().contains("not SemVer"), "{error}");
    }
    // Nothing past the well-known fetch was requested by the strict client.
    let paths: Vec<_> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.url.path().to_owned())
        .collect();
    assert!(
        paths.iter().all(|path| path == "/.well-known/tea"),
        "{paths:?}"
    );

    let releases = discover(&client(&server, Some(true))).await.unwrap();
    assert_eq!(
        releases[0].api_version, "1",
        "the advertised spelling is kept"
    );
    let paths: Vec<_> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.url.path().to_owned())
        .collect();
    assert!(paths.contains(&"/v1/discovery".to_owned()), "{paths:?}");
}

#[tokio::test]
async fn same_major_versions_need_the_flag() {
    for version in ["1.0.1", "1.1.0"] {
        let server = server(version).await;
        let off = discover(&client(&server, None)).await.unwrap_err();
        assert!(
            off.to_string().contains("supports"),
            "{version}: unexpected error {off}"
        );
        let on = discover(&client(&server, Some(true))).await.unwrap();
        assert_eq!(on[0].api_version, version);
    }
}

#[tokio::test]
async fn exact_version_works_with_and_without_the_flag() {
    let server = server("1.0.0").await;
    for flag in [None, Some(false), Some(true)] {
        let releases = discover(&client(&server, flag)).await.unwrap();
        assert_eq!(releases[0].api_version, "1.0.0");
    }
}

#[tokio::test]
async fn other_majors_are_rejected_even_when_lenient() {
    let server = server("2.0.0").await;
    let error = discover(&client(&server, Some(true))).await.unwrap_err();
    assert!(error.to_string().contains("supports"), "{error}");
}

#[tokio::test]
async fn service_accepts_a_short_version_only_when_lenient() {
    let server = MockServer::start().await;
    let strict = client(&server, None);
    assert!(strict.service(&server.uri(), "1").is_err());
    let lenient = client(&server, Some(true));
    let service = lenient.service(&server.uri(), "1").unwrap();
    assert_eq!(service.api_version(), "1");
}
