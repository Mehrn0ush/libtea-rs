//! `download --signature` reports a signature's format and never claims it is verified.

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
