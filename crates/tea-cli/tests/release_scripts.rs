//! `scripts/check-release-tag.sh` and `scripts/package-release.py`, the two
//! scripts the release workflow runs on untrusted-ish input (the tag name) and
//! on build output (the archives).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn python() -> Option<&'static str> {
    Command::new("python3")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| "python3")
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-rel-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn check_tag(tag: &str) -> Output {
    Command::new("bash")
        .arg(root().join("scripts/check-release-tag.sh"))
        .arg(tag)
        .output()
        .unwrap()
}

#[test]
fn the_tag_must_match_the_cli_version_and_a_strict_shape() {
    let version = std::fs::read_to_string(root().join("crates/tea-cli/Cargo.toml"))
        .unwrap()
        .lines()
        .find_map(|l| {
            l.strip_prefix("version = \"")
                .and_then(|v| v.strip_suffix('"'))
        })
        .unwrap()
        .to_owned();

    let real = check_tag(&format!("v{version}"));
    assert!(real.status.success());
    assert_eq!(
        String::from_utf8_lossy(&real.stdout),
        format!("version={version}\ndry_run=false\n")
    );
    let dry = check_tag(&format!("v{version}-dryrun.1"));
    assert!(dry.status.success());
    // The version keeps the suffix, so archive names and the attestation's
    // tag ref (refs/tags/v<version>) agree exactly.
    assert_eq!(
        String::from_utf8_lossy(&dry.stdout),
        format!("version={version}-dryrun.1\ndry_run=true\n")
    );

    for bad in [
        "v9.9.9".to_owned(),
        version.clone(),
        format!("V{version}"),
        format!("v{version}-"),
        format!("v{version};id"),
        format!("v{version}-x$(id)"),
        format!("v{version}\nversion=9"),
        format!("v{version}-a b"),
        format!("v{version}/../x"),
        String::new(),
    ] {
        let output = check_tag(&bad);
        assert_eq!(output.status.code(), Some(1), "{bad:?}");
        assert!(
            output.stdout.is_empty(),
            "{bad:?} printed {:?}",
            output.stdout
        );
    }
}

fn package(dir: &Path, binary: &Path, target: &str, out: &str) -> Output {
    Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/package-release.py"))
        .args(["--target", target, "--version", "0.1.0", "--binary"])
        .arg(binary)
        .args(["--out"])
        .arg(dir.join(out))
        .args(["--files"])
        .arg(root().join("LICENSE"))
        .arg(root().join("README.md"))
        .output()
        .unwrap()
}

#[test]
fn archives_are_deterministic_and_carry_no_host_identity() {
    let Some(py) = python() else { return };
    let dir = Dir::new("pkg");
    let binary = dir.0.join("libtea");
    std::fs::write(&binary, b"\x7fELF not really").unwrap();

    let first = package(&dir.0, &binary, "x86_64-unknown-linux-musl", "a");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let a = PathBuf::from(String::from_utf8(first.stdout).unwrap().trim());
    assert_eq!(
        a.file_name().unwrap(),
        "libtea-0.1.0-x86_64-unknown-linux-musl.tar.gz"
    );
    // Touch the input so a changed mtime would show up.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&binary, b"\x7fELF not really").unwrap();
    let second = package(&dir.0, &binary, "x86_64-unknown-linux-musl", "b");
    let b = PathBuf::from(String::from_utf8(second.stdout).unwrap().trim());
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

    let listing = Command::new(py)
        .arg("-I")
        .arg("-c")
        .arg(
            r#"
import sys, tarfile
t = tarfile.open(sys.argv[1])
for m in t.getmembers():
    print(m.name, oct(m.mode), m.uid, m.gid, repr(m.uname), repr(m.gname), m.mtime)
"#,
        )
        .arg(&a)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&listing.stdout).into_owned();
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 4, "{text}");
    assert!(rows[0].starts_with("libtea-0.1.0-x86_64-unknown-linux-musl "));
    assert!(rows[1].starts_with("libtea-0.1.0-x86_64-unknown-linux-musl/LICENSE 0o644 0 0 '' '' 0"));
    assert!(rows[2].starts_with("libtea-0.1.0-x86_64-unknown-linux-musl/README.md 0o644"));
    assert!(
        rows[3].starts_with("libtea-0.1.0-x86_64-unknown-linux-musl/libtea 0o755 0 0 '' '' 0"),
        "{text}"
    );
    // gzip header: no file name flag, zero mtime.
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    assert_eq!(bytes[3], 0, "FLG must be 0 (no name)");
    assert_eq!(&bytes[4..8], &[0, 0, 0, 0], "gzip mtime must be 0");
}

#[test]
fn windows_gets_a_zip_with_an_exe_and_bad_arguments_are_refused() {
    let Some(py) = python() else { return };
    let dir = Dir::new("zip");
    let binary = dir.0.join("libtea.exe");
    std::fs::write(&binary, b"MZ").unwrap();
    let output = package(&dir.0, &binary, "x86_64-pc-windows-msvc", "out");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let zip = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert_eq!(
        zip.file_name().unwrap(),
        "libtea-0.1.0-x86_64-pc-windows-msvc.zip"
    );
    let names = Command::new(py)
        .arg("-I")
        .arg("-c")
        .arg("import sys, zipfile; print('\\n'.join(i.filename for i in zipfile.ZipFile(sys.argv[1]).infolist()))")
        .arg(&zip)
        .output()
        .unwrap();
    let names = String::from_utf8_lossy(&names.stdout).into_owned();
    assert!(
        names.contains("libtea-0.1.0-x86_64-pc-windows-msvc/libtea.exe"),
        "{names}"
    );

    for target in [
        "../evil",
        "x86_64",
        "a b-c-d",
        "x86_64-unknown-linux-gnu/../x",
    ] {
        let output = package(&dir.0, &binary, target, "bad");
        assert!(!output.status.success(), "{target}");
    }
    assert!(
        !dir.0.join("bad").exists(),
        "no output for a refused target"
    );
}
