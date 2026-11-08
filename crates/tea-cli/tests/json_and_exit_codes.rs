//! `--json` output field names and the documented exit statuses
//! (`docs/cli-json.md`, `docs/cli-exit-codes.md`).

use std::path::PathBuf;
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ID2: &str = "4cc5e0f3-5d6f-4d1a-8e5a-0e9d2f4a9b11";
const BODY: &[u8] = b"tea";
// SHA-1("tea")
const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn libtea(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .env("RUST_LOG", "off")
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .output()
        .unwrap()
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes)
        .unwrap_or_else(|err| panic!("not JSON ({err}): {}", String::from_utf8_lossy(bytes)))
}

fn keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-cli-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn artifact_server(checksums: serde_json::Value, body_digest_ok: bool) -> MockServer {
    let server = MockServer::start().await;
    let format = if checksums.is_null() {
        serde_json::json!({"mediaType": "application/json"})
    } else {
        serde_json::json!({"mediaType": "application/json", "checksums": checksums})
    };
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID, "version": 1, "type": "BOM",
            "createdDate": "2024-01-01T00:00:00Z", "formats": [format]
        })))
        .mount(&server)
        .await;
    let served: &[u8] = if body_digest_ok { BODY } else { b"tampered" };
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_raw(served, "application/json"),
        )
        .mount(&server)
        .await;
    server
}

fn download(server: &MockServer, extra: &[&str]) -> Output {
    let uri = server.uri();
    let mut args = vec![
        "download",
        "--base",
        &uri,
        "--artifact",
        ID,
        "--revision",
        "1",
        "--allow-origin",
        &uri,
    ];
    args.extend_from_slice(extra);
    libtea(&args)
}

#[test]
fn spec_and_parse_tei_json_field_names() {
    let spec = json(&libtea(&["spec", "--json"]).stdout);
    assert_eq!(
        keys(&spec),
        [
            "branch",
            "commit",
            "commit_date",
            "openapi",
            "repository",
            "retrieval_date",
            "spec_version"
        ]
    );
    let tei = json(
        &libtea(&[
            "parse-tei",
            "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
            "--json",
        ])
        .stdout,
    );
    assert_eq!(
        keys(&tei),
        [
            "domain",
            "identifier",
            "publication_issues",
            "type",
            "well_known"
        ]
    );
}

#[test]
fn conformance_accepts_json_and_reports_no_certification() {
    let output = libtea(&["conformance", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let report = json(&output.stdout);
    assert_eq!(report["is_certification"], false);
    assert!(report["cases"].is_array());
}

#[tokio::test]
async fn product_release_json_is_the_release_object() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID, "product": ID2, "version": "2.0.0",
            "createdDate": "2024-01-01T00:00:00Z", "components": [],
            "latestCollection": {"uuid": ID, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
                "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"}, "artifacts": []}
        })))
        .mount(&server)
        .await;
    let uri = server.uri();
    let output = libtea(&[
        "product-release",
        "--base",
        &uri,
        "--uuid",
        ID,
        "--allow-origin",
        &uri,
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = json(&output.stdout);
    assert_eq!(value["uuid"], ID);
    assert_eq!(value["version"], "2.0.0");
}

#[tokio::test]
async fn download_json_fields_and_success_status() {
    let server = artifact_server(
        serde_json::json!([{"algType": "SHA-256", "algValue": sha256_hex(BODY)}]),
        true,
    )
    .await;
    let dir = Dir::new("dl");
    let out = dir.file("a.json");
    let output = download(&server, &["--output", &out, "--json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = json(&output.stdout);
    assert_eq!(
        keys(&value),
        [
            "artifact",
            "bytes_written",
            "content_encoding",
            "content_type",
            "etag",
            "final_url",
            "integrity",
            "media_type",
            "output",
            "revision",
            "schema_version",
            "strong_etag"
        ]
    );
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["bytes_written"], 3);
    assert_eq!(value["integrity"]["outcome"], "Verified");
    assert_eq!(keys(&value["integrity"]), ["checks", "outcome", "scope"]);
    assert_eq!(
        keys(&value["integrity"]["checks"][0]),
        [
            "actual",
            "algorithm",
            "expected",
            "matched",
            "supported",
            "weak"
        ]
    );
    assert!(value["integrity"]["scope"]
        .as_str()
        .unwrap()
        .contains("not publisher authenticity"));
    assert_eq!(std::fs::read(&out).unwrap(), BODY);
}

#[tokio::test]
async fn download_json_final_url_carries_no_query() {
    let server = artifact_server(
        serde_json::json!([{"algType": "SHA-256", "algValue": sha256_hex(BODY)}]),
        true,
    )
    .await;
    let dir = Dir::new("url");
    let out = dir.file("a.json");
    let output = download(&server, &["--output", &out, "--json"]);
    let value = json(&output.stdout);
    let url = value["final_url"].as_str().unwrap();
    assert!(!url.contains('?') && !url.contains('#'), "{url}");
}

#[test]
fn download_json_without_output_is_a_usage_error() {
    let output = libtea(&[
        "download",
        "--base",
        "http://127.0.0.1:1",
        "--artifact",
        ID,
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn bad_input_is_exit_2_and_json_error_goes_to_stderr() {
    let plain = libtea(&["parse-tei", "not-a-tei"]);
    assert_eq!(plain.status.code(), Some(2));
    let output = libtea(&["parse-tei", "not-a-tei", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "stdout must stay empty on failure"
    );
    let value = json(&output.stderr);
    assert_eq!(keys(&value), ["error", "schema_version"]);
    assert_eq!(keys(&value["error"]), ["exit_code", "kind", "message"]);
    assert_eq!(value["error"]["kind"], "usage");
    assert_eq!(value["error"]["exit_code"], 2);
    let unknown_flag = libtea(&["spec", "--nope"]);
    assert_eq!(unknown_flag.status.code(), Some(2));
}

#[tokio::test]
async fn integrity_failure_is_exit_6_and_leaves_no_file() {
    let server = artifact_server(
        serde_json::json!([{"algType": "SHA-256", "algValue": sha256_hex(BODY)}]),
        false,
    )
    .await;
    let dir = Dir::new("bad");
    let out = dir.file("a.json");
    let output = download(&server, &["--output", &out, "--json"]);
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(json(&output.stderr)["error"]["kind"], "integrity");
    assert!(!std::path::Path::new(&out).exists());
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
    // stdout mode uses the same status
    assert_eq!(download(&server, &[]).status.code(), Some(6));
}

#[tokio::test]
async fn weak_only_is_exit_4() {
    let server = artifact_server(
        serde_json::json!([{"algType": "SHA-1", "algValue": SHA1_TEA}]),
        true,
    )
    .await;
    let output = download(&server, &[]);
    assert_eq!(output.status.code(), Some(4));
    let json_error = download(
        &server,
        &["--output", &Dir::new("weak").file("a"), "--json"],
    );
    assert_eq!(json_error.status.code(), Some(4));
    assert_eq!(json(&json_error.stderr)["error"]["kind"], "weak_checksums");
}

#[tokio::test]
async fn require_checksums_without_declared_checksums_is_exit_6() {
    let server = artifact_server(serde_json::Value::Null, true).await;
    let output = download(&server, &["--require-checksums"]);
    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(download(&server, &[]).status.code(), Some(0));
}

#[tokio::test]
async fn not_found_is_exit_5_and_auth_is_exit_3() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
        )
        .mount(&server)
        .await;
    let output = download(&server, &[]);
    assert_eq!(output.status.code(), Some(5));

    let denied = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(403))
        .mount(&denied)
        .await;
    let output = download(
        &denied,
        &["--output", &Dir::new("auth").file("a"), "--json"],
    );
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(json(&output.stderr)["error"]["kind"], "auth");
}

#[tokio::test]
async fn an_existing_output_file_is_exit_1() {
    let server = artifact_server(serde_json::json!([]), true).await;
    let dir = Dir::new("exists");
    let out = dir.file("a.json");
    std::fs::write(&out, b"x").unwrap();
    let output = download(&server, &["--output", &out]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn help_documents_every_exit_status() {
    let help = String::from_utf8_lossy(&libtea(&["--help"]).stdout).into_owned();
    for expected in [
        "0 success",
        "1 failure",
        "2 usage",
        "3 authentication",
        "4 only weak",
        "5 not found",
        "6 integrity",
        "7 changes detected",
    ] {
        assert!(
            help.contains(expected),
            "--help lacks `{expected}`:\n{help}"
        );
    }
}
