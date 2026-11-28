//! `scripts/make-oci-image.py`, which `.github/workflows/image.yml` runs on the
//! verified musl release archives. The image must be the same bytes every time
//! (anyone can rebuild it and compare digests), hold only the binary, and
//! refuse archives that are not the ones it expects.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};

const VERSION: &str = "1.2.3";
const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
const AMD64: &str = "x86_64-unknown-linux-musl";
const ARM64: &str = "aarch64-unknown-linux-musl";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn have_python() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-oci-{tag}-{}", std::process::id()));
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

/// A file that passes the script's ELF check for `machine` (62 x86-64, 183
/// AArch64), followed by `tail`.
fn fake_elf(machine: u16, tail: &[u8]) -> Vec<u8> {
    let mut bytes = b"\x7fELF\x02\x01".to_vec();
    bytes.resize(64, 0);
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes.extend_from_slice(tail);
    bytes
}

/// A release archive made by `scripts/package-release.py`, as release.yml does.
fn archive(dir: &Path, target: &str, binary: &[u8], files: &[PathBuf]) -> PathBuf {
    let bin = dir.join(format!("bin-{target}"));
    std::fs::write(&bin, binary).unwrap();
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/package-release.py"))
        .args(["--target", target, "--version", VERSION, "--binary"])
        .arg(&bin)
        .arg("--out")
        .arg(dir.join(format!("dist-{target}")))
        .arg("--files")
        .args(files)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
}

fn release_files() -> Vec<PathBuf> {
    vec![root().join("LICENSE"), root().join("README.md")]
}

fn make_image(args: &[&str], amd64: &Path, arm64: &Path, out: &Path) -> Output {
    Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/make-oci-image.py"))
        .args(args)
        .arg("--amd64")
        .arg(amd64)
        .arg("--arm64")
        .arg(arm64)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

fn build(amd64: &Path, arm64: &Path, out: &Path) -> Output {
    make_image(
        &["--version", VERSION, "--revision", REVISION],
        amd64,
        arm64,
        out,
    )
}

/// Every file under `dir`, by relative path.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let name = path.strip_prefix(dir).unwrap().display().to_string();
                files.insert(name, std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// The blob a descriptor names, after checking its digest and size.
fn blob(layout: &Path, descriptor: &Value, media_type: &str) -> Vec<u8> {
    assert_eq!(descriptor["mediaType"], media_type, "{descriptor}");
    let digest = descriptor["digest"].as_str().unwrap();
    let hex = digest.strip_prefix("sha256:").unwrap();
    let bytes = std::fs::read(layout.join("blobs/sha256").join(hex)).unwrap();
    assert_eq!(sha256(&bytes), digest);
    assert_eq!(descriptor["size"], bytes.len());
    bytes
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

/// The one entry of an uncompressed ustar layer: name, octal mode, uid, gid,
/// mtime, type flag, contents.
fn only_entry(layer: &[u8]) -> (String, String, String, String, String, u8, Vec<u8>) {
    let field = |from: usize, len: usize| {
        String::from_utf8(layer[from..from + len].to_vec())
            .unwrap()
            .trim_end_matches('\0')
            .trim()
            .to_owned()
    };
    let size = usize::from_str_radix(&field(124, 12), 8).unwrap();
    let contents = layer[512..512 + size].to_vec();
    // After the entry's blocks, only zeros: no second entry.
    let end = 512 + size.div_ceil(512) * 512;
    assert!(layer[end..].iter().all(|b| *b == 0), "a second entry");
    assert_eq!(&layer[257..262], b"ustar");
    (
        field(0, 100),
        field(100, 8),
        field(108, 8),
        field(116, 8),
        field(136, 12),
        layer[156],
        contents,
    )
}

#[test]
fn the_image_is_the_same_bytes_every_time_and_holds_only_the_binary() {
    if !have_python() {
        return;
    }
    let dir = Dir::new("good");
    let amd64_binary = fake_elf(62, b"amd64 binary");
    let arm64_binary = fake_elf(183, b"arm64 binary");
    let amd64 = archive(&dir.0, AMD64, &amd64_binary, &release_files());
    let arm64 = archive(&dir.0, ARM64, &arm64_binary, &release_files());

    let first = build(&amd64, &arm64, &dir.0.join("one"));
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = build(&amd64, &arm64, &dir.0.join("two"));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(tree(&dir.0.join("one")), tree(&dir.0.join("two")));

    let layout = dir.0.join("one");
    let digest = String::from_utf8(first.stdout).unwrap().trim().to_owned();
    assert_eq!(
        std::fs::read_to_string(layout.join("oci-layout")).unwrap(),
        r#"{"imageLayoutVersion":"1.0.0"}"#
    );
    // index.json names one image, tagged with the version, whose digest is
    // what the script printed.
    let top = json(&std::fs::read(layout.join("index.json")).unwrap());
    let tagged = top["manifests"].as_array().unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0]["digest"], digest.as_str());
    assert_eq!(
        tagged[0]["annotations"]["org.opencontainers.image.ref.name"],
        VERSION
    );
    let index = json(&blob(
        &layout,
        &tagged[0],
        "application/vnd.oci.image.index.v1+json",
    ));
    let labels = BTreeMap::from([
        (
            "org.opencontainers.image.description",
            "libtea, the Transparency Exchange API (TEA) client, from the attested release archives",
        ),
        ("org.opencontainers.image.licenses", "Apache-2.0"),
        ("org.opencontainers.image.revision", REVISION),
        (
            "org.opencontainers.image.source",
            "https://github.com/Mehrn0ush/libtea-rs",
        ),
        ("org.opencontainers.image.title", "libtea"),
        ("org.opencontainers.image.version", VERSION),
    ]);
    let labels = serde_json::to_value(labels).unwrap();
    assert_eq!(index["annotations"], labels);

    let platforms = index["manifests"].as_array().unwrap();
    assert_eq!(platforms.len(), 2);
    let mut blobs = 1;
    for (platform, (arch, binary)) in platforms
        .iter()
        .zip([("amd64", &amd64_binary), ("arm64", &arm64_binary)])
    {
        assert_eq!(
            platform["platform"],
            serde_json::json!({"architecture": arch, "os": "linux"})
        );
        let manifest = json(&blob(
            &layout,
            platform,
            "application/vnd.oci.image.manifest.v1+json",
        ));
        assert_eq!(manifest["schemaVersion"], 2);
        let config = json(&blob(
            &layout,
            &manifest["config"],
            "application/vnd.oci.image.config.v1+json",
        ));
        let layers = manifest["layers"].as_array().unwrap();
        assert_eq!(layers.len(), 1);
        let layer = blob(
            &layout,
            &layers[0],
            "application/vnd.oci.image.layer.v1.tar",
        );
        blobs += 3;

        assert_eq!(config["architecture"], arch);
        assert_eq!(config["os"], "linux");
        assert_eq!(
            config["config"],
            serde_json::json!({"Entrypoint": ["/libtea"], "User": "65532:65532", "Labels": labels})
        );
        assert_eq!(
            config["rootfs"],
            serde_json::json!({"type": "layers", "diff_ids": [sha256(&layer)]})
        );
        // Nothing that records a time or the host.
        assert!(config.get("created").is_none() && config.get("history").is_none());

        let (name, mode, uid, gid, mtime, kind, contents) = only_entry(&layer);
        assert_eq!(
            (name.as_str(), mode.as_str(), uid.as_str(), gid.as_str()),
            ("libtea", "0000555", "0000000", "0000000")
        );
        assert_eq!(u64::from_str_radix(&mtime, 8).unwrap(), 0);
        assert_eq!(kind, b'0', "a regular file");
        assert_eq!(
            &contents, binary,
            "the binary from the {arch} archive, unchanged"
        );
    }
    assert_eq!(
        tree(&layout).len(),
        blobs + 2,
        "blobs plus oci-layout and index.json"
    );
}

#[test]
fn the_script_refuses_archives_it_does_not_expect() {
    if !have_python() {
        return;
    }
    let dir = Dir::new("bad");
    let amd64 = archive(&dir.0, AMD64, &fake_elf(62, b"a"), &release_files());
    let arm64 = archive(&dir.0, ARM64, &fake_elf(183, b"b"), &release_files());
    // Named for amd64 but holding an AArch64 binary.
    let wrong_arch_dir = dir.0.join("wrong-arch");
    std::fs::create_dir(&wrong_arch_dir).unwrap();
    let wrong_arch = archive(
        &wrong_arch_dir,
        AMD64,
        &fake_elf(183, b"a"),
        &release_files(),
    );
    let not_elf_dir = dir.0.join("not-elf");
    std::fs::create_dir(&not_elf_dir).unwrap();
    let not_elf = archive(
        &not_elf_dir,
        AMD64,
        b"#!/bin/sh\necho hi\n",
        &release_files(),
    );
    // One more file than release.yml packages.
    let extra_dir = dir.0.join("extra");
    std::fs::create_dir(&extra_dir).unwrap();
    let mut files = release_files();
    files.push(root().join("SECURITY.md"));
    let extra = archive(&extra_dir, AMD64, &fake_elf(62, b"a"), &files);
    // README.md missing.
    let short_dir = dir.0.join("short");
    std::fs::create_dir(&short_dir).unwrap();
    let short = archive(
        &short_dir,
        AMD64,
        &fake_elf(62, b"a"),
        &[root().join("LICENSE")],
    );

    let cases: [(&str, &Path, &Path); 6] = [
        ("swapped", &arm64, &amd64),
        ("same archive twice", &amd64, &amd64),
        ("wrong architecture", &wrong_arch, &arm64),
        ("not ELF", &not_elf, &arm64),
        ("extra file", &extra, &arm64),
        ("missing file", &short, &arm64),
    ];
    for (case, a, b) in cases {
        let out_dir = dir.0.join(format!("out-{}", case.replace(' ', "-")));
        let out = build(a, b, &out_dir);
        assert!(!out.status.success(), "{case}: accepted");
        assert!(out.stdout.is_empty(), "{case}");
        assert!(!out_dir.exists(), "{case}: wrote {}", out_dir.display());
    }

    for (case, args) in [
        (
            "prerelease",
            ["--version", "1.2.3-rc.1", "--revision", REVISION],
        ),
        ("v prefix", ["--version", "v1.2.3", "--revision", REVISION]),
        (
            "short revision",
            ["--version", VERSION, "--revision", "0123456"],
        ),
        (
            "uppercase revision",
            [
                "--version",
                VERSION,
                "--revision",
                "0123456789ABCDEF0123456789ABCDEF01234567",
            ],
        ),
    ] {
        let out_dir = dir.0.join("out-args");
        let out = make_image(&args, &amd64, &arm64, &out_dir);
        assert!(!out.status.success(), "{case}: accepted");
        assert!(!out_dir.exists(), "{case}");
    }

    // An existing output directory is never written into.
    let existing = dir.0.join("existing");
    std::fs::create_dir(&existing).unwrap();
    let out = build(&amd64, &arm64, &existing);
    assert!(!out.status.success());
    assert_eq!(std::fs::read_dir(&existing).unwrap().count(), 0);
}
