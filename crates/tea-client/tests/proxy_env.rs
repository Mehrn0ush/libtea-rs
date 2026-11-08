//! Environment proxies are honored by default and ignored on request.
//!
//! This binary changes process environment variables, so it holds one test.

use std::time::Duration;

use libtea_client::{Client, ClientBuilder, Limits};
use libtea_core::{DestinationPolicy, ProductId};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const PROXY_VARS: [&str; 8] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

async fn product_server(name: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/product/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "name": name,
            "identifiers": []
        })))
        .mount(&server)
        .await;
    server
}

fn builder(target: &MockServer) -> ClientBuilder {
    Client::builder()
        .limits(
            Limits::default()
                .with_initial_backoff(Duration::ZERO)
                .with_max_retries(0)
                .with_request_timeout(Duration::from_secs(5)),
        )
        .destination_policy(
            DestinationPolicy::allow_origin(&Url::parse(&target.uri()).unwrap()).unwrap(),
        )
}

async fn product_name(client: &Client, target: &MockServer) -> String {
    client
        .service(&target.uri(), "1.0.0")
        .unwrap()
        .get_product(ProductId::parse(ID).unwrap())
        .await
        .unwrap()
        .name
}

#[tokio::test]
async fn env_proxy_is_used_by_default_and_skipped_with_ignore_env_proxies() {
    let target = product_server("direct").await;
    let proxy = product_server("via-proxy").await;
    let saved: Vec<_> = PROXY_VARS
        .iter()
        .map(|name| (*name, std::env::var_os(name)))
        .collect();
    for name in PROXY_VARS {
        std::env::remove_var(name);
    }
    std::env::set_var("HTTP_PROXY", proxy.uri());
    std::env::set_var("http_proxy", proxy.uri());

    // Built while the variables are set: reqwest reads them at build time.
    let honoring = builder(&target).build().unwrap();
    let ignoring = builder(&target).ignore_env_proxies(true).build().unwrap();

    let via = product_name(&honoring, &target).await;
    let direct = product_name(&ignoring, &target).await;

    for (name, value) in saved {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
    assert_eq!(via, "via-proxy", "default client honors HTTP_PROXY");
    assert_eq!(
        direct, "direct",
        "ignore_env_proxies(true) connects directly"
    );
    assert!(!target.received_requests().await.unwrap().is_empty());
    assert_eq!(proxy.received_requests().await.unwrap().len(), 1);
}
