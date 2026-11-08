//! `--cache-dir`, `--offline`, `LIBTEA_CACHE_DIR` and `libtea cache stat|clear`.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const ART: &str = "11111111-1111-4111-8111-111111111111";
const TEI: &str = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const BODY: &[u8] = b"cached artifact bytes";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-cc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn cache(&self) -> PathBuf {
        self.0.join("cache")
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn libtea(args: &[String], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_libtea"));
    command
        .args(args)
        .env("RUST_LOG", "off")
        .env_remove("LIBTEA_CACHE_DIR")
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET");
    for (k, v) in env {
        command.env(k, v);
    }
    command.output().unwrap()
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|p| (*p).to_owned()).collect()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

async fn mount_all(server: &MockServer) {
    server.reset().await;
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
                "productReleaseUuid": REL,
                "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]
            }])),
        )
        .mount(server)
        .await;
    let artifact = serde_json::json!({
        "uuid": ART, "version": 1, "type": "BOM", "createdDate": "2024-01-01T00:00:00Z",
        "formats": [{"mediaType": "application/octet-stream",
            "checksums": [{"algType": "SHA-256", "algValue": sha256_hex(BODY)}]}]
    });
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/productRelease/{REL}")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"r1\"")
                .set_body_json(serde_json::json!({
                    "uuid": REL, "product": REL, "version": "1.0.0",
                    "createdDate": "2024-01-01T00:00:00Z", "components": [],
                    "latestCollection": {"uuid": REL, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
                        "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"}, "artifacts": []}
                })),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/productRelease/{REL}/collection/latest"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"c1\"")
                .set_body_json(serde_json::json!({
                    "uuid": REL, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
                    "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"},
                    "artifacts": [artifact.clone()]
                })),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ART}/1")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"a1\"")
                .set_body_json(artifact),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ART}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"d1\"")
                .set_body_raw(BODY, "application/octet-stream"),
        )
        .mount(server)
        .await;
}

async fn make_unreachable(server: &MockServer) {
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(server)
        .await;
}

fn release_args(server: &MockServer, extra: &[&str]) -> Vec<String> {
    let mut args = strings(&[
        "product-release",
        "--base",
        &server.uri(),
        "--uuid",
        REL,
        "--allow-origin",
        &server.uri(),
        "--json",
    ]);
    args.extend(extra.iter().map(|a| (*a).to_owned()));
    args
}

#[tokio::test]
async fn a_cache_dir_fills_and_offline_answers_without_the_network() {
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("offline");
    let cache = dir.cache().display().to_string();
    let online = libtea(&release_args(&server, &["--cache-dir", &cache]), &[]);
    assert_eq!(online.status.code(), Some(0), "{}", text(&online.stderr));

    let stat = libtea(
        &strings(&["cache", "stat", "--cache-dir", &cache, "--json"]),
        &[],
    );
    assert_eq!(stat.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&stat.stdout).unwrap();
    assert_eq!(value["entries"], 1);
    assert_eq!(value["exists"], true);

    make_unreachable(&server).await;
    let offline = libtea(
        &release_args(&server, &["--cache-dir", &cache, "--offline"]),
        &[],
    );
    assert_eq!(offline.status.code(), Some(0), "{}", text(&offline.stderr));
    assert_eq!(offline.stdout, online.stdout);
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "offline must not connect"
    );

    // Without --offline the broken server is a failure, the cache does not hide it.
    let live = libtea(&release_args(&server, &["--cache-dir", &cache]), &[]);
    assert_ne!(live.status.code(), Some(0));
}

#[tokio::test]
async fn the_environment_variable_selects_the_cache_directory() {
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("env");
    let cache = dir.cache().display().to_string();
    let first = libtea(&release_args(&server, &[]), &[("LIBTEA_CACHE_DIR", &cache)]);
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    make_unreachable(&server).await;
    let second = libtea(
        &release_args(&server, &["--offline"]),
        &[("LIBTEA_CACHE_DIR", &cache)],
    );
    assert_eq!(second.status.code(), Some(0), "{}", text(&second.stderr));
    assert_eq!(second.stdout, first.stdout);
}

#[tokio::test]
async fn offline_without_a_cache_directory_is_a_usage_error() {
    let server = MockServer::start().await;
    for extra in [vec!["--offline"], vec!["--cache-authenticated"]] {
        let out = libtea(&release_args(&server, &extra), &[]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{extra:?}: {}",
            text(&out.stderr)
        );
    }
}

#[tokio::test]
async fn an_offline_miss_is_reported_as_such() {
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("miss");
    let cache = dir.cache().display().to_string();
    let out = libtea(
        &release_args(&server, &["--cache-dir", &cache, "--offline"]),
        &[],
    );
    assert_eq!(out.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("offline"));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn download_and_sync_work_offline_from_a_warm_cache() {
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("warm");
    let cache = dir.cache().display().to_string();
    let lock = dir.file("tea.lock");
    let base = |extra: &[&str]| {
        let mut args = strings(&[
            "sync",
            TEI,
            "--lock",
            &lock.display().to_string(),
            "--well-known-url",
            &format!("{}/.well-known/tea", server.uri()),
            "--allow-origin",
            &server.uri(),
            "--cache-dir",
            &cache,
            "--fetched-at",
            "2026-11-08T00:00:00Z",
        ]);
        args.extend(extra.iter().map(|a| (*a).to_owned()));
        args
    };
    let online = libtea(&base(&[]), &[]);
    assert_eq!(online.status.code(), Some(0), "{}", text(&online.stderr));
    let first = std::fs::read(&lock).unwrap();

    // `download` needs the artifact's own metadata, which sync did not fetch:
    // warm that too while the server is up.
    let warm_out = dir.file("warm.bin");
    let warm = libtea(
        &strings(&[
            "download",
            "--base",
            &server.uri(),
            "--artifact",
            ART,
            "--revision",
            "1",
            "--allow-origin",
            &server.uri(),
            "--cache-dir",
            &cache,
            "--output",
            &warm_out.display().to_string(),
        ]),
        &[],
    );
    assert_eq!(warm.status.code(), Some(0), "{}", text(&warm.stderr));

    make_unreachable(&server).await;
    let offline = libtea(&base(&["--offline"]), &[]);
    assert_eq!(offline.status.code(), Some(0), "{}", text(&offline.stderr));
    assert_eq!(std::fs::read(&lock).unwrap(), first);
    let check = libtea(&base(&["--offline", "--check"]), &[]);
    assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
    // Offline, "up to date" would claim more than was checked: it compared the
    // lock with the cache, and the server was never asked.
    let said = text(&check.stdout);
    assert!(said.contains("cached responses"), "{said}");
    assert!(said.contains("server was not contacted"), "{said}");
    assert!(!said.contains("up to date"), "{said}");
    assert!(server.received_requests().await.unwrap().is_empty());

    // download --output, offline, from the same cache
    let out = dir.file("artifact.bin");
    let download = libtea(
        &strings(&[
            "download",
            "--base",
            &server.uri(),
            "--artifact",
            ART,
            "--revision",
            "1",
            "--allow-origin",
            &server.uri(),
            "--cache-dir",
            &cache,
            "--offline",
            "--output",
            &out.display().to_string(),
        ]),
        &[],
    );
    assert_eq!(
        download.status.code(),
        Some(0),
        "{}",
        text(&download.stderr)
    );
    assert_eq!(std::fs::read(&out).unwrap(), BODY);
}

#[tokio::test]
async fn stat_and_clear_report_and_only_touch_entries() {
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("tools");
    let cache = dir.cache().display().to_string();

    let missing = libtea(
        &strings(&["cache", "stat", "--cache-dir", &cache, "--json"]),
        &[],
    );
    let value: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(value["exists"], false);
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "bytes",
            "dir",
            "entries",
            "exists",
            "foreign",
            "newest_age_secs",
            "oldest_age_secs",
            "schema_version"
        ]
    );

    libtea(&release_args(&server, &["--cache-dir", &cache]), &[]);
    std::fs::write(Path::new(&cache).join("v1").join("mine.txt"), b"keep").unwrap();
    let text_stat = libtea(&strings(&["cache", "stat", "--cache-dir", &cache]), &[]);
    let shown = text(&text_stat.stdout);
    assert!(
        shown.contains("entries 1") && shown.contains("foreign 1"),
        "{shown}"
    );

    let cleared = libtea(
        &strings(&["cache", "clear", "--cache-dir", &cache, "--json"]),
        &[],
    );
    let value: serde_json::Value = serde_json::from_slice(&cleared.stdout).unwrap();
    assert_eq!(value["removed"], 1);
    assert_eq!(
        std::fs::read(Path::new(&cache).join("v1").join("mine.txt")).unwrap(),
        b"keep"
    );

    // No directory given: usage error.
    let none = libtea(&strings(&["cache", "stat"]), &[]);
    assert_eq!(none.status.code(), Some(2));
}

#[tokio::test]
async fn an_unsafe_cache_directory_is_refused_by_every_command() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let server = MockServer::start().await;
    mount_all(&server).await;
    let dir = Dir::new("unsafe");
    let real = dir.file("real");
    std::fs::create_dir(&real).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
    let link = dir.file("link");
    symlink(&real, &link).unwrap();
    let open = dir.file("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
    for bad in [&link, &open] {
        let arg = bad.display().to_string();
        let out = libtea(&release_args(&server, &["--cache-dir", &arg]), &[]);
        assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
        assert!(server.received_requests().await.unwrap().is_empty());
        let stat = libtea(&strings(&["cache", "stat", "--cache-dir", &arg]), &[]);
        assert_eq!(stat.status.code(), Some(1));
        let clear = libtea(&strings(&["cache", "clear", "--cache-dir", &arg]), &[]);
        assert_eq!(clear.status.code(), Some(1));
    }
    assert_eq!(std::fs::read_dir(&real).unwrap().count(), 0);
}

#[test]
fn help_describes_the_cache_options() {
    let out = libtea(&strings(&["product-release", "--help"]), &[]);
    let help = text(&out.stdout);
    for flag in ["--cache-dir", "--offline", "--cache-authenticated"] {
        assert!(help.contains(flag), "{flag} missing:\n{help}");
    }
    let out = libtea(&strings(&["cache", "--help"]), &[]);
    let help = text(&out.stdout);
    assert!(help.contains("stat") && help.contains("clear"), "{help}");
}
