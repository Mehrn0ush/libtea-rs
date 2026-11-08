//! `libtea sync`, `sync --check` and `libtea diff` against a mock TEA server.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REL: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const A1: &str = "11111111-1111-4111-8111-111111111111";
const A2: &str = "22222222-2222-4222-8222-222222222222";
const TEI: &str = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const SECRET: &str = "s3cr3t-value-that-must-not-leak";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One format the mock collection publishes.
#[derive(Clone)]
struct Entry {
    uuid: &'static str,
    revision: u64,
    media: &'static str,
    /// What the server serves.
    body: &'static [u8],
    /// What the metadata declares; `None` means the digest of `body`.
    declared_for: Option<&'static [u8]>,
    declare_checksum: bool,
}

impl Entry {
    fn new(uuid: &'static str, media: &'static str, body: &'static [u8]) -> Self {
        Self {
            uuid,
            revision: 1,
            media,
            body,
            declared_for: None,
            declare_checksum: true,
        }
    }
}

async fn mount(server: &MockServer, collection_version: u64, entries: &[Entry]) {
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
    let artifacts: Vec<_> = entries
        .iter()
        .map(|entry| {
            let declared = sha256_hex(entry.declared_for.unwrap_or(entry.body));
            let mut format = serde_json::json!({"mediaType": entry.media});
            if entry.declare_checksum {
                format["checksums"] =
                    serde_json::json!([{"algType": "SHA-256", "algValue": declared}]);
            }
            serde_json::json!({
                "uuid": entry.uuid,
                "version": entry.revision,
                "type": "BOM",
                "createdDate": "2024-01-01T00:00:00Z",
                "formats": [format]
            })
        })
        .collect();
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/productRelease/{REL}/collection/latest"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": REL,
            "version": collection_version,
            "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "PRODUCT_RELEASE",
            "updateReason": {"type": "INITIAL_RELEASE"},
            "artifacts": artifacts
        })))
        .mount(server)
        .await;
    for entry in entries {
        Mock::given(method("GET"))
            .and(path(format!(
                "/v1.0.0/artifact/{}/{}/download",
                entry.uuid, entry.revision
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(entry.body, "application/octet-stream"),
            )
            .mount(server)
            .await;
    }
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-sync-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn libtea(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(args)
        .env("RUST_LOG", "off")
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .output()
        .unwrap()
}

fn sync(server: &MockServer, lock: &Path, extra: &[&str]) -> Output {
    let mut args: Vec<String> = vec![
        "sync".into(),
        TEI.into(),
        "--lock".into(),
        lock.display().to_string(),
        "--well-known-url".into(),
        format!("{}/.well-known/tea", server.uri()),
        "--allow-origin".into(),
        server.uri(),
    ];
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    libtea(&args)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn two() -> Vec<Entry> {
    vec![
        Entry::new(A2, "text/plain", b"second"),
        Entry::new(A1, "application/json", b"{\"first\":1}"),
    ]
}

#[tokio::test]
async fn sync_writes_a_sorted_lock_with_the_documented_fields() {
    let server = MockServer::start().await;
    mount(&server, 2, &two()).await;
    let dir = Dir::new("write");
    let lock = dir.file("tea.lock");
    let output = sync(&server, &lock, &["--fetched-at", "2026-11-08T00:00:00Z"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let value = read_json(&lock);
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["tei"], TEI);
    assert_eq!(value["apiVersion"], "1.0.0");
    assert_eq!(
        value["specCommit"],
        "962a0228af1bfcc1232ac3f79239317ff33fdb04"
    );
    assert_eq!(value["productRelease"], REL);
    assert_eq!(value["collectionVersion"], 2);
    assert!(value["server"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1"));
    let artifacts = value["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 2);
    assert_eq!(artifacts[0]["uuid"], A1, "sorted by uuid");
    assert_eq!(artifacts[0]["mediaType"], "application/json");
    assert_eq!(artifacts[0]["revision"], 1);
    assert_eq!(artifacts[0]["type"], "BOM");
    assert_eq!(artifacts[0]["size"], 11);
    assert_eq!(artifacts[0]["integrity"], "Verified");
    assert_eq!(artifacts[0]["fetchedAt"], "2026-11-08T00:00:00Z");
    assert_eq!(artifacts[0]["checksums"][0]["algorithm"], "SHA-256");
    assert_eq!(
        artifacts[0]["checksums"][0]["value"],
        sha256_hex(b"{\"first\":1}")
    );
    assert_eq!(dir.names(), ["tea.lock"], "no temporary file may remain");
}

#[tokio::test]
async fn the_same_content_gives_the_same_bytes_and_an_unchanged_lock_is_not_rewritten() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("same");
    let lock = dir.file("tea.lock");
    assert_eq!(
        sync(&server, &lock, &["--fetched-at", "2026-11-08T00:00:00Z"])
            .status
            .code(),
        Some(0)
    );
    let first = std::fs::read(&lock).unwrap();
    // A second run with another timestamp must leave the file alone.
    let again = sync(
        &server,
        &lock,
        &["--fetched-at", "2030-01-01T00:00:00Z", "--json"],
    );
    assert_eq!(again.status.code(), Some(0), "{}", text(&again.stderr));
    assert_eq!(std::fs::read(&lock).unwrap(), first);
    let summary: serde_json::Value = serde_json::from_slice(&again.stdout).unwrap();
    assert_eq!(summary["written"], false);
    assert_eq!(summary["previous"], "unchanged");
    // A fresh lock elsewhere is byte-identical.
    let other = dir.file("other.lock");
    sync(&server, &other, &["--fetched-at", "2026-11-08T00:00:00Z"]);
    assert_eq!(std::fs::read(&other).unwrap(), first);
}

#[tokio::test]
async fn check_passes_then_reports_added_removed_and_changed_with_exit_7() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("check");
    let lock = dir.file("tea.lock");
    sync(&server, &lock, &[]);
    let before = std::fs::read(&lock).unwrap();

    let ok = sync(&server, &lock, &["--check"]);
    assert_eq!(ok.status.code(), Some(0), "{}", text(&ok.stderr));
    assert!(text(&ok.stdout).contains("up to date"));

    // added
    let mut entries = two();
    entries.push(Entry::new(
        "33333333-3333-4333-8333-333333333333",
        "text/plain",
        b"third",
    ));
    mount(&server, 2, &entries).await;
    let added = sync(&server, &lock, &["--check", "--json"]);
    assert_eq!(added.status.code(), Some(7), "{}", text(&added.stderr));
    let value: serde_json::Value = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(value["has_changes"], true);
    assert_eq!(value["added"].as_array().unwrap().len(), 1);
    assert_eq!(value["header"][0]["field"], "collectionVersion");
    let error: serde_json::Value = serde_json::from_slice(&added.stderr).unwrap();
    assert_eq!(error["error"]["kind"], "changes_detected");

    // removed
    mount(&server, 1, &two()[..1]).await;
    let removed = sync(&server, &lock, &["--check"]);
    assert_eq!(removed.status.code(), Some(7));
    assert!(
        text(&removed.stdout).contains(&format!("- {A1}"))
            || text(&removed.stdout).contains(&format!("- {A2}"))
    );

    // changed: a new revision and different bytes
    let mut changed = two();
    changed[0].revision = 2;
    changed[0].body = b"second, edited";
    mount(&server, 1, &changed).await;
    let modified = sync(&server, &lock, &["--check", "--json"]);
    assert_eq!(modified.status.code(), Some(7));
    let value: serde_json::Value = serde_json::from_slice(&modified.stdout).unwrap();
    let fields: Vec<_> = value["modified"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(fields, ["revision", "size", "checksums"]);

    assert_eq!(
        std::fs::read(&lock).unwrap(),
        before,
        "--check never writes"
    );
}

#[tokio::test]
async fn tampered_bytes_fail_closed_and_leave_the_lock_alone() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("tamper");
    let lock = dir.file("tea.lock");

    // No lock yet: nothing may be created.
    let mut bad = two();
    bad[1].declared_for = Some(b"what the publisher declared");
    mount(&server, 1, &bad).await;
    let output = sync(&server, &lock, &["--json"]);
    assert_eq!(output.status.code(), Some(6), "{}", text(&output.stderr));
    assert!(output.stdout.is_empty());
    assert!(!lock.exists());
    assert!(dir.names().is_empty(), "left behind: {:?}", dir.names());

    // An existing lock stays byte-for-byte as it was, with and without --check.
    mount(&server, 1, &two()).await;
    sync(&server, &lock, &[]);
    let before = std::fs::read(&lock).unwrap();
    mount(&server, 1, &bad).await;
    assert_eq!(sync(&server, &lock, &[]).status.code(), Some(6));
    assert_eq!(std::fs::read(&lock).unwrap(), before);
    let check = sync(&server, &lock, &["--check"]);
    assert_eq!(
        check.status.code(),
        Some(6),
        "tampering is an integrity failure, not a diff"
    );
    assert_eq!(dir.names(), ["tea.lock"]);
}

#[tokio::test]
async fn a_format_without_checksums_is_recorded_as_not_declared() {
    let server = MockServer::start().await;
    let mut entries = two();
    entries[0].declare_checksum = false;
    mount(&server, 1, &entries).await;
    let dir = Dir::new("nodecl");
    let lock = dir.file("tea.lock");
    assert_eq!(sync(&server, &lock, &[]).status.code(), Some(0));
    let value = read_json(&lock);
    let by_uuid = |uuid: &str| {
        value["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["uuid"] == uuid)
            .unwrap()
            .clone()
    };
    assert_eq!(by_uuid(A2)["integrity"], "NotDeclared");
    assert_eq!(by_uuid(A2)["checksums"].as_array().unwrap().len(), 0);
    assert_eq!(by_uuid(A1)["integrity"], "Verified");
}

#[tokio::test]
async fn artifacts_dir_keeps_the_verified_bytes_under_sanitised_names() {
    let server = MockServer::start().await;
    mount(
        &server,
        1,
        &[Entry::new(A1, "application/json", b"{\"first\":1}")],
    )
    .await;
    let dir = Dir::new("keep");
    let lock = dir.file("tea.lock");
    let store = dir.file("store");
    let output = sync(
        &server,
        &lock,
        &["--artifacts-dir", &store.display().to_string()],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let kept = std::fs::read_dir(&store)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("{A1}-r1-application_json-"))
        })
        .expect("a file named for the artifact, revision and media type");
    assert_eq!(std::fs::read(&kept).unwrap(), b"{\"first\":1}");
    assert_eq!(
        std::fs::read_dir(&store).unwrap().count(),
        1,
        "no temporary files"
    );
}

#[tokio::test]
async fn no_credential_reaches_the_lock_or_the_output() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("secret");
    let lock = dir.file("tea.lock");
    let mut args: Vec<String> = vec![
        "sync".into(),
        TEI.into(),
        "--lock".into(),
        lock.display().to_string(),
        "--well-known-url".into(),
        format!("{}/.well-known/tea", server.uri()),
        "--allow-origin".into(),
        server.uri(),
    ];
    args.push("--json".into());
    let output = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(&args)
        .env("RUST_LOG", "off")
        .env("TEA_API_KEY_ID", "key-id-value")
        .env("TEA_API_KEY_SECRET", SECRET)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    for haystack in [
        text(&output.stdout),
        text(&output.stderr),
        text(&std::fs::read(&lock).unwrap()),
    ] {
        assert!(!haystack.contains(SECRET) && !haystack.contains("key-id-value"));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_symlinked_lock_is_never_written_through() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("link");
    // A valid lock sits behind the link, so reading it succeeds and only the
    // write can be refused.
    let target = dir.file("target.lock");
    assert_eq!(sync(&server, &target, &[]).status.code(), Some(0));
    let before = std::fs::read(&target).unwrap();
    let lock = dir.file("tea.lock");
    std::os::unix::fs::symlink(&target, &lock).unwrap();
    mount(&server, 2, &two()).await;
    let output = sync(&server, &lock, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(
        text(&output.stderr).contains("symbolic link"),
        "{}",
        text(&output.stderr)
    );
    assert_eq!(std::fs::read(&target).unwrap(), before);
    assert!(std::fs::symlink_metadata(&lock)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(dir.names(), ["target.lock", "tea.lock"]);
}

#[tokio::test]
async fn ambiguous_releases_and_a_missing_check_lock_are_usage_errors() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("usage");
    let missing = sync(&server, &dir.file("none.lock"), &["--check"]);
    assert_eq!(missing.status.code(), Some(2), "{}", text(&missing.stderr));

    // Two product releases behind one TEI.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/.well-known/tea"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "schemaVersion": 1,
            "endpoints": [{"url": server.uri(), "versions": ["1.0.0"], "priority": 1}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1.0.0/discovery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"productReleaseUuid": REL,
             "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]},
            {"productReleaseUuid": "4cc5e0f3-5d6f-4d1a-8e5a-0e9d2f4a9b11",
             "servers": [{"rootUrl": server.uri(), "versions": ["1.0.0"], "priority": 1}]}
        ])))
        .mount(&server)
        .await;
    let output = sync(&server, &dir.file("a.lock"), &[]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("--product-release"));
    assert!(!dir.file("a.lock").exists());
}

fn lock_with(dir: &Dir, name: &str, edit: impl Fn(&mut serde_json::Value)) -> PathBuf {
    let mut value = serde_json::json!({
        "schemaVersion": 1,
        "tei": TEI,
        "server": "https://tea.example.com/",
        "apiVersion": "1.0.0",
        "specCommit": "962a0228af1bfcc1232ac3f79239317ff33fdb04",
        "productRelease": REL,
        "collectionVersion": 1,
        "artifacts": [{
            "uuid": A1, "revision": 1, "type": "BOM", "mediaType": "text/plain",
            "size": 3, "integrity": "Verified", "fetchedAt": "2026-11-08T00:00:00Z",
            "checksums": [{"algorithm": "SHA-256", "value": sha256_hex(b"abc")}]
        }]
    });
    edit(&mut value);
    let path = dir.file(name);
    std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    path
}

#[test]
fn diff_compares_two_locks_offline() {
    let dir = Dir::new("diff");
    let old = lock_with(&dir, "old.lock", |_| {});
    let same = lock_with(&dir, "same.lock", |v| {
        v["artifacts"][0]["fetchedAt"] = "2031-01-01T00:00:00Z".into();
    });
    let new = lock_with(&dir, "new.lock", |v| {
        v["artifacts"][0]["size"] = 4.into();
        v["collectionVersion"] = 2.into();
    });
    let args = |a: &Path, b: &Path, json: bool| {
        let mut v = vec![
            "diff".to_owned(),
            a.display().to_string(),
            b.display().to_string(),
        ];
        if json {
            v.push("--json".into());
        }
        v
    };
    let equal = libtea(&args(&old, &same, false));
    assert_eq!(equal.status.code(), Some(0), "{}", text(&equal.stderr));
    assert!(text(&equal.stdout).contains("no differences"));

    let differ = libtea(&args(&old, &new, true));
    assert_eq!(differ.status.code(), Some(7));
    let value: serde_json::Value = serde_json::from_slice(&differ.stdout).unwrap();
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "added",
            "has_changes",
            "header",
            "modified",
            "removed",
            "schema_version"
        ]
    );
    assert_eq!(value["modified"][0]["fields"][0]["field"], "size");
    assert_eq!(value["header"][0]["old"], "1");
    assert_eq!(value["header"][0]["new"], "2");
}

#[test]
fn diff_rejects_bad_locks_with_exit_2() {
    let dir = Dir::new("badlock");
    let good = lock_with(&dir, "good.lock", |_| {});
    let cases = [
        lock_with(&dir, "v2.lock", |v| v["schemaVersion"] = 2.into()),
        lock_with(&dir, "extra.lock", |v| v["surprise"] = true.into()),
        lock_with(&dir, "secret.lock", |v| {
            v["server"] = "https://user:pw@tea.example.com/".into()
        }),
        lock_with(&dir, "dup.lock", |v| {
            let first = v["artifacts"][0].clone();
            v["artifacts"].as_array_mut().unwrap().push(first);
        }),
    ];
    for bad in cases {
        let output = libtea(&[
            "diff".into(),
            good.display().to_string(),
            bad.display().to_string(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}: {}",
            bad.display(),
            text(&output.stderr)
        );
    }
    let missing = libtea(&[
        "diff".into(),
        good.display().to_string(),
        dir.file("nope").display().to_string(),
    ]);
    assert_eq!(missing.status.code(), Some(2));
    let garbage = dir.file("garbage.lock");
    std::fs::write(&garbage, b"\x00\x01 not json").unwrap();
    let output = libtea(&[
        "diff".into(),
        good.display().to_string(),
        garbage.display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(2));
}

/// A mock with one artifact that has several formats, each served for its own
/// `mediaType` query.
async fn mount_formats(server: &MockServer, formats: &[(&'static str, &'static [u8])]) {
    mount(server, 1, &[]).await;
    let artifact_formats: Vec<_> = formats
        .iter()
        .map(|(media, body)| {
            serde_json::json!({
                "mediaType": media,
                "checksums": [{"algType": "SHA-256", "algValue": sha256_hex(body)}]
            })
        })
        .collect();
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1.0.0/productRelease/{REL}/collection/latest"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "uuid": REL, "version": 1, "createdDate": "2024-01-01T00:00:00Z",
            "belongsTo": "PRODUCT_RELEASE", "updateReason": {"type": "INITIAL_RELEASE"},
            "artifacts": [{
                "uuid": A1, "version": 1, "type": "BOM",
                "createdDate": "2024-01-01T00:00:00Z", "formats": artifact_formats
            }]
        })))
        .with_priority(1)
        .mount(server)
        .await;
    for (media, body) in formats {
        Mock::given(method("GET"))
            .and(path(format!("/v1.0.0/artifact/{A1}/1/download")))
            .and(query_param("mediaType", *media))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(*body, "application/octet-stream"),
            )
            .with_priority(1)
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn media_types_that_slug_alike_do_not_overwrite_each_other() {
    let server = MockServer::start().await;
    mount_formats(
        &server,
        &[
            ("application/x y", b"space"),
            ("application/x_y", b"underscore"),
        ],
    )
    .await;
    let dir = Dir::new("collide");
    let lock = dir.file("tea.lock");
    let store = dir.file("store");
    let output = sync(
        &server,
        &lock,
        &["--artifacts-dir", &store.display().to_string()],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let mut contents: Vec<Vec<u8>> = std::fs::read_dir(&store)
        .unwrap()
        .map(|e| std::fs::read(e.unwrap().path()).unwrap())
        .collect();
    contents.sort();
    assert_eq!(
        contents,
        vec![b"space".to_vec(), b"underscore".to_vec()],
        "both formats must be kept, in two files"
    );
}

#[tokio::test]
async fn server_text_with_control_characters_or_absurd_length_is_refused() {
    let long: &'static str = Box::leak(format!("text/{}", "a".repeat(300)).into_boxed_str());
    for media in ["text/\u{7}bell", "text/\u{1b}[31mred", "text/a\nb", long] {
        let server = MockServer::start().await;
        mount(&server, 1, &[Entry::new(A1, media, b"data")]).await;
        let dir = Dir::new("ctrl");
        let lock = dir.file("tea.lock");
        let output = sync(&server, &lock, &[]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{media:?}: {}",
            text(&output.stderr)
        );
        assert!(
            !lock.exists(),
            "{media:?}: a lock with hostile text was written"
        );
        assert!(
            text(&output.stderr).contains("lock field mediaType"),
            "{media:?}: {}",
            text(&output.stderr)
        );
        assert!(dir.names().is_empty());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_planted_temporary_file_cannot_block_the_lock_write() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("planted");
    let lock = dir.file("tea.lock");
    let mut args: Vec<String> = vec![
        "sync".into(),
        TEI.into(),
        "--lock".into(),
        lock.display().to_string(),
        "--well-known-url".into(),
        format!("{}/.well-known/tea", server.uri()),
        "--allow-origin".into(),
        server.uri(),
    ];
    args.push("--json".into());
    let child = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(&args)
        .env("RUST_LOG", "off")
        .env_remove("TEA_API_KEY_ID")
        .env_remove("TEA_API_KEY_SECRET")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // Plant the name an earlier version derived from the process id, and a
    // symbolic link under a few other guesses, before the lock is written.
    let pid = child.id();
    let planted = dir.file(&format!(".tea.lock.libtea-{pid}.tmp"));
    std::fs::write(&planted, b"planted").unwrap();
    let target = dir.file("victim.txt");
    std::fs::write(&target, b"do not touch").unwrap();
    for guess in 0..4u32 {
        let _ =
            std::os::unix::fs::symlink(&target, dir.file(&format!(".tea.lock.{guess:016x}.tmp")));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(lock.exists());
    assert_eq!(
        std::fs::read(&planted).unwrap(),
        b"planted",
        "the planted file was touched"
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"do not touch");
}

#[tokio::test]
async fn a_second_sync_leaves_no_temporary_files() {
    let server = MockServer::start().await;
    mount(&server, 1, &two()).await;
    let dir = Dir::new("notemp");
    let lock = dir.file("tea.lock");
    for round in 0..3 {
        mount(&server, round + 1, &two()).await;
        assert_eq!(sync(&server, &lock, &[]).status.code(), Some(0));
    }
    assert_eq!(dir.names(), ["tea.lock"]);
}
