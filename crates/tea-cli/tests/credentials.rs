//! CLI credential resolution: environment preferred; argv secret is insecure but supported.

use std::process::Command;

use base64::Engine;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const KEY_ID: &str = "cli-key-id";
const KEY_SECRET: &str = "cli-key-secret-value";

/// Match `cdx_tea_client` token Basic construction: form-encode each part, then Base64.
fn basic_authorization(id: &str, secret: &str) -> String {
    let id: String = url::form_urlencoded::byte_serialize(id.as_bytes()).collect();
    let secret: String = url::form_urlencoded::byte_serialize(secret.as_bytes()).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{id}:{secret}"));
    format!("Basic {encoded}")
}

fn product_release() -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "product": ID,
        "version": "1.0.0",
        "createdDate": "2024-01-01T00:00:00Z",
        "components": [],
        "latestCollection": {
            "uuid": ID,
            "version": 1,
            "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "PRODUCT_RELEASE",
            "updateReason": {"type": "INITIAL_RELEASE"},
            "artifacts": []
        }
    })
}

fn challenge() -> ResponseTemplate {
    ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\"")
}

#[tokio::test]
async fn environment_credentials_authenticate_product_release() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"env-token","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .and(header("authorization", "Bearer env-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_release()))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(challenge())
        .with_priority(2)
        .mount(&server)
        .await;

    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .args([
            "product-release",
            "--base",
            &server.uri(),
            "--uuid",
            ID,
            "--allow-origin",
            &server.uri(),
            "--json",
        ])
        .env("TEA_API_KEY_ID", KEY_ID)
        .env("TEA_API_KEY_SECRET", KEY_SECRET)
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1.0.0"));
    assert!(!stdout.contains(KEY_SECRET));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(KEY_SECRET));
}

#[tokio::test]
async fn environment_credentials_preserve_spaces_in_basic_header() {
    let id = " id ";
    let secret = " secret ";
    let expected = basic_authorization(id, secret);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .and(header("authorization", expected.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"spaced-env-token","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .and(header("authorization", "Bearer spaced-env-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_release()))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(challenge())
        .with_priority(2)
        .mount(&server)
        .await;

    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .args([
            "product-release",
            "--base",
            &server.uri(),
            "--uuid",
            ID,
            "--allow-origin",
            &server.uri(),
        ])
        .env("TEA_API_KEY_ID", id)
        .env("TEA_API_KEY_SECRET", secret)
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
}

#[tokio::test]
async fn cli_credentials_preserve_spaces_in_basic_header() {
    let id = " cli-id ";
    let secret = " cli-secret ";
    let expected = basic_authorization(id, secret);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .and(header("authorization", expected.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"spaced-cli-token","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .and(header("authorization", "Bearer spaced-cli-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_release()))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(challenge())
        .with_priority(2)
        .mount(&server)
        .await;

    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .args([
            "product-release",
            "--base",
            &server.uri(),
            "--uuid",
            ID,
            "--allow-origin",
            &server.uri(),
            "--api-key-id",
            id,
            "--api-key-secret",
            secret,
        ])
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .env_remove("RUST_LOG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
}

#[tokio::test]
async fn partial_environment_credentials_fail_without_leaking_secret() {
    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .args([
            "product-release",
            "--base",
            "http://127.0.0.1:9",
            "--uuid",
            ID,
            "--allow-origin",
            "http://127.0.0.1:9",
        ])
        .env("TEA_API_KEY_ID", KEY_ID)
        .env_remove("TEA_API_KEY_SECRET")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("TEA_API_KEY_SECRET"));
    assert!(!stderr.contains(KEY_SECRET));
}

#[tokio::test]
async fn collections_list_emits_json_without_mixing_diagnostics() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}/collections")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "hasNext": false,
            "results": [{
                "uuid": ID,
                "version": 1,
                "createdDate": "2024-01-01T00:00:00Z",
                "belongsTo": "PRODUCT_RELEASE",
                "updateReason": {"type": "INITIAL_RELEASE"},
                "artifacts": []
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .args([
            "collections",
            "--base",
            &server.uri(),
            "--uuid",
            ID,
            "--page-size",
            "10",
            "--allow-origin",
            &server.uri(),
            "--json",
        ])
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["results"][0]["version"], 1);
    assert!(output.stderr.is_empty() || !stdout.contains("has_next"));
}

#[test]
fn help_mentions_environment_credentials() {
    let output = Command::new(env!("CARGO_BIN_EXE_tea"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("TEA_API_KEY_ID"));
    assert!(text.contains("TEA_API_KEY_SECRET"));
    assert!(text.contains("insecure"));
}
