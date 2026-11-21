//! `download --signature` reports a signature's format; it says "verified" only for a
//! raw signature that verifies under a `--signature-key` the caller gave.

use std::path::PathBuf;
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const BODY: &[u8] = b"tea-bytes";
const ARMORED: &str =
    "-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEE\n-----END PGP SIGNATURE-----\n";

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// An artifact with one format and a verified checksum, so the only thing
/// under test is the signature report.
async fn server_with_artifact() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "version": 1,
            "type": "BOM",
            "createdDate": "2024-01-01T00:00:00Z",
            "formats": [{
                "mediaType": "application/json",
                "checksums": [{"algType": "SHA-256", "algValue": sha256(BODY)}]
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(BODY, "application/json"))
        .mount(&server)
        .await;
    server
}

async fn signature_responds(server: &MockServer, response: ResponseTemplate, times: u64) {
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/signature/download")))
        .and(query_param("mediaType", "application/json"))
        .respond_with(response)
        .expect(times)
        .mount(server)
        .await;
}

fn libtea(server: &MockServer, extra: &[&str]) -> Output {
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
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .env("RUST_LOG", "off")
        .output()
        .unwrap()
}

fn scratch_file(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("libtea-sig-{label}-{}-{nanos}", std::process::id()))
}

#[tokio::test]
async fn present_signature_is_named_and_not_verified() {
    let server = server_with_artifact().await;
    signature_responds(
        &server,
        ResponseTemplate::new(200).set_body_raw(ARMORED, "application/pgp-signature"),
        1,
    )
    .await;
    let output = libtea(&server, &["--signature"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr {stderr}");
    assert_eq!(output.stdout, BODY, "stdout carries only the artifact");
    assert!(stderr.contains("signature present\tformat openpgp_armored\tbytes "));
    assert!(stderr.contains("not verified"));
    assert!(!stderr.contains("signature verified"));
}

#[tokio::test]
async fn not_published_signature_is_reported_and_exits_zero() {
    let server = server_with_artifact().await;
    signature_responds(
        &server,
        ResponseTemplate::new(404)
            .set_body_raw(r#"{"error":"SIGNATURE_NOT_FOUND"}"#, "application/json"),
        1,
    )
    .await;
    let output = libtea(&server, &["--signature"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr {stderr}");
    assert_eq!(output.stdout, BODY);
    assert!(stderr.contains("signature not published for this format"));
}

#[tokio::test]
async fn an_unknown_artifact_on_the_signature_path_is_an_error() {
    let server = server_with_artifact().await;
    signature_responds(
        &server,
        ResponseTemplate::new(404)
            .set_body_raw(r#"{"error":"OBJECT_UNKNOWN"}"#, "application/json"),
        1,
    )
    .await;
    let output = libtea(&server, &["--signature"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("signature present"));
    assert!(!stderr.contains("signature not published"));
}

#[tokio::test]
async fn json_carries_the_signature_member_only_with_the_flag() {
    let server = server_with_artifact().await;
    // DER ContentInfo with the SignedData OID: detected as CMS, not checked.
    let cms: &[u8] = &[
        0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02, 0xa0, 0x00,
    ];
    signature_responds(
        &server,
        ResponseTemplate::new(200).set_body_raw(cms, "application/pkcs7-signature"),
        1,
    )
    .await;

    let with = scratch_file("with");
    let output = libtea(
        &server,
        &["--output", with.to_str().unwrap(), "--json", "--signature"],
    );
    let _ = std::fs::remove_file(&with);
    assert!(
        output.status.success(),
        "stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    let signature = &value["signature"];
    assert_eq!(signature["status"], "present");
    assert_eq!(signature["format"], "cms");
    assert_eq!(signature["bytes"], cms.len());
    assert_eq!(signature["content_type"], "application/pkcs7-signature");
    assert_eq!(signature["verified"], false);
    assert!(signature["final_url"]
        .as_str()
        .unwrap()
        .ends_with("/signature/download"));

    // Without the flag: no signature request (the mock expects exactly one,
    // made above) and no `signature` member.
    let without = scratch_file("without");
    let output = libtea(&server, &["--output", without.to_str().unwrap(), "--json"]);
    let _ = std::fs::remove_file(&without);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.get("signature").is_none());
}

/// The OpenSSL test vectors of `libtea-client` (`tests/fixtures/signature/`).
fn vector(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tea-client/tests/fixtures/signature")
        .join(name)
}

const ED25519_FINGERPRINT: &str =
    "sha256:9797c6e083b023da1acb7b83fd4635033203c436f2562c00012191dc3670b20f";

/// The signed fixture as the artifact, with its checksum, and its Ed25519
/// signature (or `signature` instead) on the signature path.
async fn server_with_signed_artifact(
    signature: Option<Vec<u8>>,
    signature_requests: u64,
) -> MockServer {
    let body = std::fs::read(vector("artifact.txt")).unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "version": 1,
            "type": "BOM",
            "createdDate": "2024-01-01T00:00:00Z",
            "formats": [{
                "mediaType": "application/json",
                "checksums": [{"algType": "SHA-256", "algValue": sha256(&body)}]
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    let signature =
        signature.unwrap_or_else(|| std::fs::read(vector("artifact.ed25519.sig")).unwrap());
    signature_responds(
        &server,
        ResponseTemplate::new(200).set_body_raw(signature, "application/octet-stream"),
        signature_requests,
    )
    .await;
    server
}

fn key_arg(name: &str) -> String {
    vector(name).to_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_signature_that_verifies_under_a_caller_key() {
    let server = server_with_signed_artifact(None, 2).await;
    let ed = key_arg("ed25519.pub.pem");
    let p256 = key_arg("p256.pub.pem");
    // Two keys: one is enough.
    let output = libtea(&server, &["--signature-key", &p256, "--signature-key", &ed]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr {stderr}");
    assert_eq!(
        output.stdout,
        std::fs::read(vector("artifact.txt")).unwrap()
    );
    assert!(
        stderr.contains(&format!(
            "signature verified\tcaller key ed25519 {ED25519_FINGERPRINT}"
        )),
        "{stderr}"
    );

    let file = scratch_file("verified");
    let output = libtea(
        &server,
        &[
            "--output",
            file.to_str().unwrap(),
            "--json",
            "--signature-key",
            &ed,
        ],
    );
    let _ = std::fs::remove_file(&file);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let signature = &value["signature"];
    assert_eq!(signature["verified"], true);
    assert_eq!(signature["outcome"], "verified");
    assert_eq!(
        signature["policy"],
        format!("caller key ed25519 {ED25519_FINGERPRINT}")
    );
    assert_eq!(signature["detail"], serde_json::Value::Null);
    assert_eq!(signature["saved_to"], serde_json::Value::Null);
}

#[tokio::test]
async fn a_signature_that_does_not_verify_is_exit_8_and_prints_nothing() {
    let server = server_with_signed_artifact(None, 2).await;
    let p256 = key_arg("p256.pub.pem");
    let output = libtea(&server, &["--signature-key", &p256]);
    assert_eq!(output.status.code(), Some(8));
    assert!(
        output.stdout.is_empty(),
        "nothing is printed without a verified signature"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("signature invalid\t"), "{stderr}");
    assert!(
        stderr.contains("signature not verified: the signature is invalid"),
        "{stderr}"
    );

    let file = scratch_file("invalid");
    let output = libtea(
        &server,
        &[
            "--output",
            file.to_str().unwrap(),
            "--json",
            "--signature-key",
            &p256,
        ],
    );
    let written = file.exists();
    let _ = std::fs::remove_file(&file);
    assert_eq!(output.status.code(), Some(8));
    assert!(output.stdout.is_empty());
    assert!(
        written,
        "the checksum-verified file stays, and the message says so"
    );
    // Progress lines come first; the error object is the last line.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_str(stderr.lines().last().unwrap()).unwrap();
    assert_eq!(error["error"]["kind"], "signature");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("do not use it"));
}

#[tokio::test]
async fn a_container_signature_with_a_key_is_unsupported_and_exit_8() {
    let server = server_with_signed_artifact(Some(ARMORED.as_bytes().to_vec()), 1).await;
    let output = libtea(&server, &["--signature-key", &key_arg("ed25519.pub.pem")]);
    assert_eq!(output.status.code(), Some(8));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("signature unsupported\t"), "{stderr}");
    assert!(stderr.contains("openpgp_armored"), "{stderr}");
}

#[tokio::test]
async fn a_missing_signature_with_a_key_is_exit_8() {
    let server = server_with_artifact().await;
    signature_responds(
        &server,
        ResponseTemplate::new(404)
            .set_body_raw(r#"{"error":"SIGNATURE_NOT_FOUND"}"#, "application/json"),
        1,
    )
    .await;
    let output = libtea(&server, &["--signature-key", &key_arg("ed25519.pub.pem")]);
    assert_eq!(output.status.code(), Some(8));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("signature not verified: no signature is published for this format"));
}

#[tokio::test]
async fn the_signature_bytes_can_be_saved() {
    let server = server_with_signed_artifact(None, 1).await;
    let saved = scratch_file("saved-sig");
    let output = libtea(&server, &["--signature-output", saved.to_str().unwrap()]);
    let bytes = std::fs::read(&saved);
    let _ = std::fs::remove_file(&saved);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        bytes.unwrap(),
        std::fs::read(vector("artifact.ed25519.sig")).unwrap()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("signature saved to "), "{stderr}");
    assert!(
        stderr.contains("not verified (no --signature-key given)"),
        "{stderr}"
    );
}

#[tokio::test]
async fn bad_signature_options_are_refused_before_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    // Not a key: usage error.
    let not_a_key = vector("artifact.txt");
    let output = libtea(&server, &["--signature-key", not_a_key.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A missing key file: usage error.
    let output = libtea(&server, &["--signature-key", "/nonexistent/libtea/key.pem"]);
    assert_eq!(output.status.code(), Some(2));
    // An existing --signature-output: refused like --output.
    let existing = vector("artifact.txt");
    let output = libtea(&server, &["--signature-output", existing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
}
