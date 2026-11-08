//! `download --output`: create-new by default, `--force` to replace, never
//! through a symlink, and never a file name taken from server data. Also the
//! exit status for weak-only checksums.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ID: &str = "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
const BODY: &[u8] = b"tea";
// SHA-1("tea")
const SHA1_TEA: &str = "75490f32c06a50051804339453d3fe2f0cd5564b";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn server_with(alg: &str, digest: &str) -> MockServer {
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
                "checksums": [{"algType": alg, "algValue": digest}]
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1.0.0/artifact/{ID}/1/download")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "content-disposition",
                    "attachment; filename=\"../evil.txt\"",
                )
                .set_body_raw(BODY, "application/json"),
        )
        .mount(&server)
        .await;
    server
}

async fn server() -> MockServer {
    server_with("SHA-256", &sha256_hex(BODY)).await
}

fn run(server: &MockServer, extra: &[&str]) -> Output {
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
    args.extend(extra.iter().map(|arg| (*arg).to_owned()));
    Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(&args)
        .env("RUST_LOG", "off")
        .output()
        .unwrap()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("libtea-out-{label}-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn arg(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[tokio::test]
async fn output_creates_a_new_file_and_ignores_server_file_names() {
    let server = server().await;
    let dir = Scratch::new("new");
    let out = dir.path("artifact.json");
    let output = run(&server, &["--output", arg(&out)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(std::fs::read(&out).unwrap(), BODY);
    // Content-Disposition `../evil.txt` was not used for anything.
    assert_eq!(dir.entries(), vec!["artifact.json".to_owned()]);
    assert!(!dir.0.parent().unwrap().join("evil.txt").exists());
}

#[tokio::test]
async fn an_existing_file_is_refused_without_force_and_replaced_with_it() {
    let server = server().await;
    let dir = Scratch::new("exists");
    let out = dir.path("artifact.json");
    std::fs::write(&out, b"keep me").unwrap();

    let refused = run(&server, &["--output", arg(&out)]);
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("--force"), "{}", stderr(&refused));
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");

    let forced = run(&server, &["--output", arg(&out), "--force"]);
    assert!(forced.status.success(), "{}", stderr(&forced));
    assert_eq!(std::fs::read(&out).unwrap(), BODY);
    assert_eq!(
        dir.entries(),
        vec!["artifact.json".to_owned()],
        "no temp file left"
    );
}

#[tokio::test]
async fn a_symlink_is_refused_with_and_without_force() {
    let server = server().await;
    let dir = Scratch::new("link");
    let target = dir.path("target.txt");
    std::fs::write(&target, b"do not touch").unwrap();
    let link = dir.path("link.json");
    symlink(&target, &link).unwrap();
    let dangling = dir.path("dangling.json");
    symlink(dir.path("missing.txt"), &dangling).unwrap();

    for extra in [&[][..], &["--force"][..]] {
        for path in [&link, &dangling] {
            let mut args = vec!["--output", arg(path)];
            args.extend_from_slice(extra);
            let output = run(&server, &args);
            assert!(!output.status.success(), "{args:?}");
            assert!(
                stderr(&output).contains("symbolic link"),
                "{}",
                stderr(&output)
            );
        }
    }
    assert_eq!(std::fs::read(&target).unwrap(), b"do not touch");
    assert!(!dir.path("missing.txt").exists());
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
}

/// The documentation must say what is actually checked: only the final path
/// component. (Behavior is recorded by the test below.)
#[test]
fn help_text_says_only_the_final_path_component_is_checked() {
    let output = Command::new(env!("CARGO_BIN_EXE_libtea"))
        .args(["download", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&output.stdout).replace('\n', " ");
    let help = help.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(help.contains("final path component"), "{help}");
    assert!(
        help.contains("Parent directories are not checked"),
        "{help}"
    );
    assert!(!help.contains("is always refused"), "{help}");
}

/// Only the final path component is checked. A symlinked parent directory (for
/// example `/tmp` on macOS) is followed, by decision; parent-symlink refusal is
/// deliberately not implemented.
#[tokio::test]
async fn a_symlinked_parent_directory_is_followed_by_design() {
    let server = server().await;
    let dir = Scratch::new("parent");
    let real = dir.path("real");
    std::fs::create_dir(&real).unwrap();
    let alias = dir.path("alias");
    symlink(&real, &alias).unwrap();
    let out = alias.join("artifact.json");

    let output = run(&server, &["--output", arg(&out)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(std::fs::read(real.join("artifact.json")).unwrap(), BODY);

    // The create-new rule still applies through the alias.
    let again = run(&server, &["--output", arg(&out)]);
    assert!(!again.status.success());
    let forced = run(&server, &["--output", arg(&out), "--force"]);
    assert!(forced.status.success(), "{}", stderr(&forced));
    // A final-component symlink is still refused.
    let link = real.join("link.json");
    symlink(real.join("artifact.json"), &link).unwrap();
    let refused = run(&server, &["--output", arg(&link), "--force"]);
    assert!(
        stderr(&refused).contains("symbolic link"),
        "{}",
        stderr(&refused)
    );
}

#[tokio::test]
async fn force_requires_output() {
    let server = server().await;
    let output = run(&server, &["--force"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[tokio::test]
async fn weak_only_checksums_exit_4_unless_allowed() {
    let server = server_with("SHA-1", SHA1_TEA).await;
    let dir = Scratch::new("weak");
    let out = dir.path("artifact.json");

    let weak = run(&server, &["--output", arg(&out), "--require-checksums"]);
    assert_eq!(weak.status.code(), Some(4), "{}", stderr(&weak));
    assert!(stderr(&weak).contains("--allow-weak-checksums"));
    assert!(!out.exists(), "nothing is written for a weak-only match");

    let allowed = run(
        &server,
        &[
            "--output",
            arg(&out),
            "--require-checksums",
            "--allow-weak-checksums",
        ],
    );
    assert!(allowed.status.success(), "{}", stderr(&allowed));
    let message = stderr(&allowed);
    assert!(message.contains("weak algorithms only"), "{message}");
    assert!(message.contains("not publisher authenticity"), "{message}");
    assert_eq!(std::fs::read(&out).unwrap(), BODY);
}
