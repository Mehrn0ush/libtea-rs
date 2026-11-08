//! CLI download writes artifact bytes to stdout and status to stderr.

use std::process::Command;

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn artifact(digest: &str) -> serde_json::Value {
    serde_json::json!({
        "uuid": ID,
        "version": 1,
        "type": "BOM",
        "createdDate": "2024-01-01T00:00:00Z",
        "formats": [{
            "mediaType": "application/json",
            "checksums": [{"algType": "SHA-256", "algValue": digest}]
        }]
    })
}

#[tokio::test]
async fn stdout_contains_only_verified_artifact_bytes() {
    let body = b"tea-bytes";
    let digest = sha256(body);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(artifact(&digest)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let output = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args([
            "download",
            "--base",
            &server.uri(),
            "--artifact",
            ID,
            "--revision",
            "1",
            "--allow-origin",
            &server.uri(),
        ])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, body);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("integrity verified")
            && stderr.contains("decoded-byte checksums only")
            && stderr.contains("not publisher authenticity")
    );
    assert!(!stderr.contains("not verified"));
}

#[tokio::test]
async fn a_wrong_checksum_does_not_write_artifact_bytes() {
    let server = MockServer::start().await;
    let digest = "0000000000000000000000000000000000000000000000000000000000000000";
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(artifact(digest)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea-bytes", "application/json"))
        .mount(&server)
        .await;
    let output = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args([
            "download",
            "--base",
            &server.uri(),
            "--artifact",
            ID,
            "--revision",
            "1",
            "--allow-origin",
            &server.uri(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[tokio::test]
async fn require_checksums_rejects_a_format_without_checksums() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "version": 1,
            "type": "BOM",
            "createdDate": "2024-01-01T00:00:00Z",
            "formats": [{"mediaType": "application/json"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea-bytes", "application/json"))
        .mount(&server)
        .await;
    let run = |require: bool| {
        let mut args = vec![
            "download".to_owned(),
            "--base".to_owned(),
            server.uri(),
            "--artifact".to_owned(),
            ID.to_owned(),
            "--revision".to_owned(),
            "1".to_owned(),
            "--allow-origin".to_owned(),
            server.uri(),
        ];
        if require {
            args.push("--require-checksums".to_owned());
        }
        Command::new(env!("CARGO_BIN_EXE_libtea"))
            .args(&args)
            .env("RUST_LOG", "off")
            .output()
            .unwrap()
    };
    // Without the flag, undeclared checksums are reported and the bytes are written.
    let lenient = run(false);
    assert!(lenient.status.success());
    assert_eq!(lenient.stdout, b"tea-bytes");
    // With the flag, the same response is a failure and nothing reaches stdout.
    let strict = run(true);
    assert!(!strict.status.success());
    assert!(strict.stdout.is_empty());
    assert!(String::from_utf8_lossy(&strict.stderr).contains("--require-checksums"));
}
