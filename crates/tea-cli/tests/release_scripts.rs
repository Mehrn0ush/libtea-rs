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

/// `cargo-cyclonedx` writes one SBOM per workspace member whichever manifest it
/// is pointed at (seen with 0.5.10 on this workspace: four files). The script
/// must keep the CLI's, remove the others, and leave nothing behind.
#[test]
fn make_sbom_keeps_the_cli_sbom_when_the_tool_writes_one_per_member() {
    if python().is_none() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = Dir::new("sbom");
    let tree = dir.0.join("tree");
    let bin = dir.0.join("bin");
    std::fs::create_dir_all(tree.join("scripts")).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    for member in ["tea-cli", "tea-client", "tea-core", "tea-conformance"] {
        std::fs::create_dir_all(tree.join("crates").join(member)).unwrap();
    }
    std::fs::copy(
        root().join("scripts/make-sbom.sh"),
        tree.join("scripts/make-sbom.sh"),
    )
    .unwrap();
    let write_exec = |name: &str, body: &str| {
        let path = bin.join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    write_exec("cargo-cyclonedx", "#!/bin/sh\nexit 0\n");
    write_exec(
        "cargo",
        r#"#!/bin/sh
[ "$1" = cyclonedx ] || exit 2
for m in cli client core conformance; do
  printf '{"bomFormat":"CycloneDX","specVersion":"1.5","metadata":{"component":{"name":"libtea-%s"}},"components":[{"name":"x"}]}' "$m" \
    > "crates/tea-$m/libtea-$m.cdx.json"
done
"#,
    );
    let out = dir.0.join("out/sbom.json");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = Command::new("bash")
        .arg(tree.join("scripts/make-sbom.sh"))
        .arg(&out)
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("\"name\":\"libtea-cli\""), "{text}");
    for member in ["tea-cli", "tea-client", "tea-core", "tea-conformance"] {
        let left: Vec<_> = std::fs::read_dir(tree.join("crates").join(member))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(left.is_empty(), "{member}: left behind {left:?}");
    }
}

/// `key = "value"` from one `[section]` of the CLI's Cargo.toml.
fn binstall_value(section: &str, key: &str) -> String {
    let manifest = std::fs::read_to_string(root().join("crates/tea-cli/Cargo.toml")).unwrap();
    let mut current = "";
    for line in manifest.lines() {
        if line.starts_with('[') {
            current = line;
        } else if current == section {
            if let Some(rest) = line.strip_prefix(&format!("{key} = ")) {
                return rest.trim_matches('"').to_owned();
            }
        }
    }
    panic!("{key} missing from {section}");
}

fn render(template: &str, target: &str, ext: &str) -> String {
    template
        .replace("{ repo }", "https://github.com/Mehrn0ush/libtea-rs")
        .replace("{ version }", "0.1.0")
        .replace("{ target }", target)
        .replace("{ bin }", "libtea")
        .replace("{ binary-ext }", ext)
}

#[test]
fn binstall_metadata_names_the_release_archives() {
    let Some(py) = python() else { return };
    let base = "[package.metadata.binstall]";
    let windows = "[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]";
    assert_eq!(binstall_value(base, "pkg-fmt"), "tgz");
    assert_eq!(binstall_value(windows, "pkg-fmt"), "zip");
    assert_eq!(
        binstall_value(base, "disabled-strategies"),
        r#"["quick-install"]"#
    );
    let dir = Dir::new("binstall");
    for (target, url_key, ext) in [
        ("x86_64-unknown-linux-musl", base, ""),
        ("x86_64-pc-windows-msvc", windows, ".exe"),
    ] {
        let binary = dir.0.join(format!("libtea{ext}"));
        std::fs::write(&binary, b"not really").unwrap();
        let out = package(&dir.0, &binary, target, target);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let archive = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
        let url = render(&binstall_value(url_key, "pkg-url"), target, ext);
        assert_eq!(
            url,
            format!(
                "https://github.com/Mehrn0ush/libtea-rs/releases/download/v0.1.0/{}",
                archive.file_name().unwrap().to_str().unwrap()
            )
        );
        let bin = render(&binstall_value(base, "bin-dir"), target, ext);
        let listing = Command::new(py)
            .arg("-I")
            .arg("-c")
            .arg(
                r#"
import sys, tarfile, zipfile
p = sys.argv[1]
names = zipfile.ZipFile(p).namelist() if p.endswith(".zip") else tarfile.open(p).getnames()
print("\n".join(names))
"#,
            )
            .arg(&archive)
            .output()
            .unwrap();
        let names = String::from_utf8_lossy(&listing.stdout).into_owned();
        assert!(names.lines().any(|n| n == bin), "{bin} not in {names}");
    }
}

fn homebrew_formula(dir: &Path, version: &str, sums: &str) -> Output {
    let path = dir.join("SHA256SUMS");
    std::fs::write(&path, sums).unwrap();
    Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/make-homebrew-formula.py"))
        .args(["--version", version, "--sums"])
        .arg(&path)
        .output()
        .unwrap()
}

#[test]
fn homebrew_formula_uses_the_darwin_and_musl_archives_from_sha256sums() {
    if python().is_none() {
        return;
    }
    let dir = Dir::new("brew");
    let targets = [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "aarch64-unknown-linux-musl",
        "x86_64-unknown-linux-musl",
    ];
    let mut sums = String::new();
    for (index, target) in targets.iter().enumerate() {
        sums.push_str(&format!(
            "{:064x}  libtea-0.1.0-{target}.tar.gz\n",
            index + 1
        ));
    }
    sums.push_str(&format!(
        "{:064x}  libtea-0.1.0-x86_64-unknown-linux-gnu.tar.gz\n",
        9
    ));
    sums.push_str(&format!(
        "{:064x}  libtea-0.1.0-x86_64-pc-windows-msvc.zip\n",
        9
    ));

    let out = homebrew_formula(&dir.0, "0.1.0", &sums);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let formula = String::from_utf8(out.stdout).unwrap();
    for (index, target) in targets.iter().enumerate() {
        let url = format!(
            "url \"https://github.com/Mehrn0ush/libtea-rs/releases/download/v0.1.0/libtea-0.1.0-{target}.tar.gz\"\n      sha256 \"{:064x}\"",
            index + 1
        );
        assert!(formula.contains(&url), "{target} missing:\n{formula}");
    }
    assert!(!formula.contains("gnu"), "{formula}");
    assert!(!formula.contains(&format!("{:064x}", 9)), "{formula}");
    assert!(formula.contains("  version \"0.1.0\"\n"));
    assert!(formula.contains("  license \"Apache-2.0\"\n"));
    assert!(formula.contains("bin.install \"libtea\""));

    // Each refusal: an archive missing, a bad line, a bad version.
    let missing = sums.replace("libtea-0.1.0-x86_64-apple-darwin.tar.gz", "other");
    for (version, sums) in [
        ("0.1.0", missing),
        ("0.1.0", format!("{sums}not a line\n")),
        ("0.1.0\"; system \"x", sums.clone()),
        ("v0.1.0", sums.clone()),
    ] {
        let out = homebrew_formula(&dir.0, version, &sums);
        assert!(!out.status.success(), "accepted {version:?}");
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn release_archives_unpack_to_one_directory_holding_libtea() {
    // Homebrew enters the single top-level directory before `bin.install
    // "libtea"`, so the formula depends on this layout.
    let Some(py) = python() else { return };
    let dir = Dir::new("brewpkg");
    let binary = dir.0.join("libtea");
    std::fs::write(&binary, b"not really").unwrap();
    let out = package(&dir.0, &binary, "aarch64-apple-darwin", "out");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let archive = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
    let listing = Command::new(py)
        .arg("-I")
        .arg("-c")
        .arg("import sys, tarfile; print('\\n'.join(tarfile.open(sys.argv[1]).getnames()))")
        .arg(&archive)
        .output()
        .unwrap();
    let names = String::from_utf8_lossy(&listing.stdout).into_owned();
    let tops: std::collections::BTreeSet<&str> = names
        .lines()
        .map(|n| n.split('/').next().unwrap())
        .collect();
    assert_eq!(tops.len(), 1, "{names}");
    assert!(names
        .lines()
        .any(|n| n == "libtea-0.1.0-aarch64-apple-darwin/libtea"));
}
