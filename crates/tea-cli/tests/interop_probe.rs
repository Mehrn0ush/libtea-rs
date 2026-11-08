//! End-to-end run of `scripts/interop-probe.sh` against a loopback wiremock stub.
//!
//! The loopback origin is reached through the script's documented
//! `TEA_ALLOW_ORIGIN` switch (passed to the CLI as `--allow-origin`).

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const KEY_ID: &str = "probe-key-id-7";
// Glob metacharacters: an unquoted `${text//$secret/...}` would not match this literally.
const SECRET: &str = "pa*ss?wo[r]d-SECRET";
const TEI: &str = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/interop-probe.sh")
}

fn scratch_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "libtea-probe-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn run_probe(envs: &[(&str, String)]) -> Output {
    let mut command = Command::new("bash");
    command.arg(script()).env_clear();
    for name in [
        "PATH",
        "HOME",
        "TMPDIR",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
    ] {
        if let Ok(value) = std::env::var(name) {
            command.env(name, value);
        }
    }
    command.env("TEA_CLI_BIN", env!("CARGO_BIN_EXE_libtea"));
    command.env("RUST_LOG", "off");
    for (name, value) in envs {
        command.env(name, value);
    }
    command.output().expect("bash runs the probe script")
}

fn result_files(dir: &Path) -> (PathBuf, PathBuf) {
    let mut json = None;
    let mut md = None;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => json = Some(path),
            Some("md") => md = Some(path),
            _ => {}
        }
    }
    (json.expect("json result"), md.expect("md result"))
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn assert_secrets_absent(text: &str) {
    assert!(!text.contains(SECRET), "secret leaked: {text}");
    assert!(!text.contains("wo[r]d"), "secret fragment leaked: {text}");
    assert!(!text.contains(KEY_ID), "key id leaked: {text}");
}

async fn mount_happy_server(server: &MockServer) {
    let body = b"probe-bytes";
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "productReleaseUuid": ID,
                "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]
            }])),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"probe-token","token_type":"Bearer","expires_in":3600}"#,
            "application/json",
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .and(header("authorization", "Bearer probe-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(product_release()))
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .with_priority(2)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": ID,
            "version": 1,
            "type": "BOM",
            "createdDate": "2024-01-01T00:00:00Z",
            "formats": [{
                "mediaType": "application/json",
                "checksums": [{"algType": "SHA-256", "algValue": sha256(body)}]
            }]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "application/json"))
        .mount(server)
        .await;
}

#[tokio::test]
async fn probe_runs_end_to_end_and_keeps_secrets_out_of_results() {
    let server = MockServer::start().await;
    mount_happy_server(&server).await;
    let out = scratch_dir("ok");
    let output = run_probe(&[
        ("TEA_TEI", TEI.into()),
        (
            "TEA_WELL_KNOWN_URL",
            format!("{}/.well-known/tea", server.uri()),
        ),
        ("TEA_BASE_URL", server.uri()),
        ("TEA_PRODUCT_RELEASE", ID.into()),
        ("TEA_ARTIFACT_UUID", ID.into()),
        ("TEA_ALLOW_ORIGIN", server.uri()),
        ("TEA_API_KEY_ID", KEY_ID.into()),
        ("TEA_API_KEY_SECRET", SECRET.into()),
        ("TEA_OUT_DIR", out.display().to_string()),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stdout {stdout}\nstderr {stderr}");
    assert_secrets_absent(&stdout);
    assert_secrets_absent(&stderr);

    assert_eq!(mode(&out), 0o700, "created output directory is private");
    let (json_path, md_path) = result_files(&out);
    assert_eq!(mode(&json_path), 0o600);
    assert_eq!(mode(&md_path), 0o600);
    let json_text = std::fs::read_to_string(&json_path).unwrap();
    let md_text = std::fs::read_to_string(&md_path).unwrap();
    assert_secrets_absent(&json_text);
    assert_secrets_absent(&md_text);

    let result: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    assert_eq!(result["pass"], 3, "{json_text}");
    assert_eq!(result["fail"], 0, "{json_text}");
    assert_eq!(result["spec_commit"], libtea_core::spec::COMMIT);
    assert_ne!(result["client_version"], "unknown");
    assert_eq!(result["auth_mode"], "api_key_env");
    assert_eq!(result["checksum_result"], "checksum_match_decoded_bytes");
    assert_eq!(result["server_base_url"], server.uri());

    let requests = server.received_requests().await.unwrap();
    assert!(requests
        .iter()
        .any(|request| request.method.as_str() == "POST"));
    for request in &requests {
        let allowed = matches!(request.method.as_str(), "GET" | "HEAD")
            || (request.method.as_str() == "POST" && request.url.path() == "/v1.0.0/token");
        assert!(
            allowed,
            "unexpected {} {}",
            request.method,
            request.url.path()
        );
    }
    std::fs::remove_dir_all(&out).unwrap();
}

#[tokio::test]
async fn probe_redacts_a_secret_echoed_by_the_server_and_strips_url_parts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1.0.0/token"))
        .respond_with(ResponseTemplate::new(400).set_body_raw(
            serde_json::json!({"error": format!("echo {SECRET} {KEY_ID}")}).to_string(),
            "application/json",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{ID}")))
        .respond_with(
            ResponseTemplate::new(401).insert_header("www-authenticate", "Bearer realm=\"tea\""),
        )
        .mount(&server)
        .await;
    // Pre-existing directory: its mode must be left alone.
    let out = scratch_dir("echo");
    std::fs::create_dir(&out).unwrap();
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755)).unwrap();
    let output = run_probe(&[
        ("TEA_BASE_URL", server.uri()),
        ("TEA_PRODUCT_RELEASE", ID.into()),
        ("TEA_ALLOW_ORIGIN", server.uri()),
        ("TEA_API_KEY_ID", KEY_ID.into()),
        ("TEA_API_KEY_SECRET", SECRET.into()),
        ("TEA_OUT_DIR", out.display().to_string()),
    ]);
    assert_eq!(output.status.code(), Some(1), "an operation failed");
    assert_eq!(mode(&out), 0o755, "existing directory mode unchanged");
    let (json_path, _) = result_files(&out);
    let json_text = std::fs::read_to_string(&json_path).unwrap();
    assert_secrets_absent(&json_text);
    assert!(json_text.contains("[redacted]"), "{json_text}");
    assert!(json_text.contains("[redacted-id]"), "{json_text}");
    std::fs::remove_dir_all(&out).unwrap();

    // Userinfo, query, and fragment never reach the result file.
    let out = scratch_dir("url");
    let port = server.address().port();
    let output = run_probe(&[
        (
            "TEA_BASE_URL",
            format!("http://alice:hunter2@127.0.0.1:{port}/tea?sig=topsecret#frag"),
        ),
        ("TEA_ARTIFACT_UUID", ID.into()),
        ("TEA_OUT_DIR", out.display().to_string()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let (json_path, md_path) = result_files(&out);
    for text in [
        std::fs::read_to_string(&json_path).unwrap(),
        std::fs::read_to_string(&md_path).unwrap(),
    ] {
        let result_url = format!("http://127.0.0.1:{port}/tea");
        assert!(text.contains(&result_url), "{text}");
        for leaked in ["alice", "hunter2", "topsecret", "frag"] {
            let in_url = text
                .lines()
                .filter(|line| line.contains("server_base_url"))
                .any(|line| line.contains(leaked));
            assert!(!in_url, "{leaked} kept in server_base_url: {text}");
        }
    }
    std::fs::remove_dir_all(&out).unwrap();
}

#[tokio::test]
async fn probe_reports_weak_only_checksums_by_exit_status() {
    // SHA-1("tea"); the mock below serves exactly "tea".
    const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";
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
                "checksums": [{"algType": "SHA-1", "algValue": SHA1_TEA}]
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(ResponseTemplate::new(200).set_body_raw("tea", "application/json"))
        .mount(&server)
        .await;
    for (allow, code, checksum) in [
        (false, 1, "weak_algorithms_only"),
        (true, 0, "checksum_match_decoded_bytes"),
    ] {
        let out = scratch_dir("weak");
        let mut envs = vec![
            ("TEA_BASE_URL", server.uri()),
            ("TEA_ARTIFACT_UUID", ID.to_owned()),
            ("TEA_ALLOW_ORIGIN", server.uri()),
            ("TEA_OUT_DIR", out.display().to_string()),
        ];
        if allow {
            envs.push(("TEA_ALLOW_WEAK_CHECKSUMS", "1".to_owned()));
        }
        let output = run_probe(&envs);
        assert_eq!(
            output.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let (json_path, _) = result_files(&out);
        let result: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(result["checksum_result"], checksum);
        assert_eq!(result["allow_weak_checksums"], allow);
        std::fs::remove_dir_all(&out).unwrap();
    }
}

#[test]
fn probe_with_nothing_to_do_fails_instead_of_passing() {
    let out = scratch_dir("empty");
    let output = run_probe(&[("TEA_OUT_DIR", out.display().to_string())]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nothing was probed"), "{stderr}");
    let written = std::fs::read_dir(&out)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(written, 0, "no result files for a vacuous run");
    let _ = std::fs::remove_dir_all(&out);
}

// ---------------------------------------------------------------------------
// The probe must never run cargo while credentials are in the environment.
// A fake `cargo` on PATH records any invocation.
// ---------------------------------------------------------------------------

/// A directory holding a `cargo` that writes `marker` and fails.
fn fake_cargo(label: &str) -> (PathBuf, PathBuf) {
    let dir = scratch_dir(label);
    std::fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("cargo-was-run");
    let script = dir.join("cargo");
    std::fs::write(
        &script,
        format!("#!/bin/sh\necho \"$@\" > '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    (dir, marker)
}

/// Like `run_probe`, but with the fake cargo first on PATH and `TEA_CLI_BIN`
/// set only when `cli_bin` is true.
fn run_probe_with_fake_cargo(envs: &[(&str, String)], fake_dir: &Path, cli_bin: bool) -> Output {
    let mut command = Command::new("bash");
    command.arg(script()).env_clear();
    let path = format!(
        "{}:{}",
        fake_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    command.env("PATH", path);
    for name in ["HOME", "TMPDIR"] {
        if let Ok(value) = std::env::var(name) {
            command.env(name, value);
        }
    }
    if cli_bin {
        command.env("TEA_CLI_BIN", env!("CARGO_BIN_EXE_libtea"));
    }
    command.env("RUST_LOG", "off");
    for (name, value) in envs {
        command.env(name, value);
    }
    command.output().expect("bash runs the probe script")
}

#[test]
fn credentials_without_a_prebuilt_cli_refuse_to_run_cargo() {
    for credentials in [
        vec![
            ("TEA_API_KEY_ID", KEY_ID.to_owned()),
            ("TEA_API_KEY_SECRET", SECRET.to_owned()),
        ],
        vec![("TEA_API_KEY_SECRET", SECRET.to_owned())],
        vec![("TEA_API_KEY_ID", KEY_ID.to_owned())],
    ] {
        let (fake, marker) = fake_cargo("refuse");
        let out = scratch_dir("refuse-out");
        let mut envs = credentials;
        envs.push(("TEA_BASE_URL", "https://tea.example".to_owned()));
        envs.push(("TEA_PRODUCT_RELEASE", ID.to_owned()));
        envs.push(("TEA_OUT_DIR", out.display().to_string()));
        let output = run_probe_with_fake_cargo(&envs, &fake, false);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert_eq!(output.status.code(), Some(2), "{stderr}");
        assert!(stderr.contains("TEA_CLI_BIN"), "{stderr}");
        assert!(stderr.contains("refusing to run cargo"), "{stderr}");
        assert!(!marker.exists(), "cargo was invoked with credentials set");
        assert_secrets_absent(&stderr);
        assert!(!out.exists() || std::fs::read_dir(&out).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&fake);
        let _ = std::fs::remove_dir_all(&out);
    }
}

#[test]
fn credentials_with_a_prebuilt_cli_never_invoke_cargo() {
    let (fake, marker) = fake_cargo("prebuilt");
    let out = scratch_dir("prebuilt-out");
    // Nothing to probe: the script gets as far as reading the pinned commit
    // from the CLI and the version from the manifest, then stops.
    let output = run_probe_with_fake_cargo(
        &[
            ("TEA_API_KEY_ID", KEY_ID.to_owned()),
            ("TEA_API_KEY_SECRET", SECRET.to_owned()),
            ("TEA_OUT_DIR", out.display().to_string()),
        ],
        &fake,
        true,
    );
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("nothing was probed"), "{stderr}");
    assert!(
        !marker.exists(),
        "cargo was invoked although TEA_CLI_BIN was set"
    );
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn without_credentials_the_script_may_still_build_with_cargo() {
    // Control: the refusal is about credentials, not about cargo in general.
    let (fake, marker) = fake_cargo("anonymous");
    let out = scratch_dir("anonymous-out");
    let output = run_probe_with_fake_cargo(
        &[
            ("TEA_BASE_URL", "https://tea.example".to_owned()),
            ("TEA_PRODUCT_RELEASE", ID.to_owned()),
            ("TEA_OUT_DIR", out.display().to_string()),
        ],
        &fake,
        false,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cargo build"));
    assert!(
        marker.exists(),
        "the anonymous path should have tried cargo"
    );
    let _ = std::fs::remove_dir_all(&fake);
    let _ = std::fs::remove_dir_all(&out);
}
