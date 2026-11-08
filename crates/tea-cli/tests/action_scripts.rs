//! The GitHub Action's scripts (`scripts/action/`), run against stubs: a fake
//! `curl` that serves a fixture release, a fake `gh`, and a fake `libtea`.
//! Nothing here touches the network.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn have(tool: &str) -> Option<PathBuf> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {tool}"))
        .output()
        .ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("libtea-act-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("stubs")).unwrap();
        fs::create_dir_all(dir.join("release")).unwrap();
        Self { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn write_exe(&self, rel: &str, body: &str) {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A PATH holding only symlinks to the tools the scripts need, plus the
    /// stubs, so a real `curl`, `gh` or `libtea` on the machine cannot leak in.
    fn restricted_path(&self, with_gh: bool) -> String {
        let bin = self.path("bin");
        fs::create_dir_all(&bin).unwrap();
        for tool in [
            "bash",
            "env",
            "awk",
            "grep",
            "sed",
            "mktemp",
            "mv",
            "rm",
            "mkdir",
            "cat",
            "dirname",
            "basename",
            "sha256sum",
            "shasum",
            "python3",
            "head",
            "tr",
            "printf",
            "wc",
            "pwd",
            "chmod",
            "cp",
            "sort",
            "uname",
            "date",
        ] {
            if let Some(real) = have(tool) {
                let link = bin.join(tool);
                if !link.exists() {
                    let _ = symlink(real, link);
                }
            }
        }
        let mut parts = vec![
            self.path("stubs").display().to_string(),
            bin.display().to_string(),
        ];
        if !with_gh {
            parts.push(String::new());
        }
        parts.retain(|p| !p.is_empty());
        parts.join(":")
    }

    /// `curl` that serves `release/<basename of url>` and logs what it saw.
    fn install_curl(&self) {
        self.write_exe(
            "stubs/curl",
            &format!(
                r#"#!/bin/bash
out=""; url=""; prev=""
for arg in "$@"; do
  if [[ "$prev" == "--output" ]]; then out="$arg"; fi
  prev="$arg"; url="$arg"
done
echo "URL $url" >> "{log}"
echo "CRED ${{TEA_API_KEY_SECRET:-unset}}/${{TEA_API_KEY_ID:-unset}}" >> "{log}"
for flag in "$@"; do echo "FLAG $flag" >> "{log}"; done
file="{release}/$(basename "$url")"
[[ -f "$file" ]] || exit 22
cp "$file" "$out"
"#,
                log = self.path("curl.log").display(),
                release = self.path("release").display()
            ),
        );
    }

    fn install_gh(&self, exit: i32) {
        self.write_exe(
            "stubs/gh",
            &format!(
                "#!/bin/bash\necho \"$@\" >> \"{}\"\nexit {exit}\n",
                self.path("gh.log").display()
            ),
        );
    }

    /// A fake `libtea` that prints each argument on its own line.
    fn install_libtea(&self, exit: i32) {
        self.write_exe(
            "stubs/libtea",
            &format!(
                r#"#!/bin/bash
for a in "$@"; do printf 'ARG:%s\n' "$a"; done
printf 'PWD:%s\n' "$(pwd)"
printf 'KEYSET:%s\n' "${{TEA_API_KEY_ID:+yes}}"
exit {exit}
"#
            ),
        );
    }

    fn log(&self, name: &str) -> String {
        fs::read_to_string(self.path(name)).unwrap_or_default()
    }

    /// Build the fixture release for `target` and write SHA256SUMS.
    fn make_release(&self, version: &str, target: &str) -> String {
        let fake = self.path("fake-libtea");
        fs::write(&fake, "#!/bin/sh\necho fixture libtea\n").unwrap();
        let out = Command::new("python3")
            .arg("-I")
            .arg(root().join("scripts/package-release.py"))
            .args(["--target", target, "--version", version, "--binary"])
            .arg(&fake)
            .args(["--out"])
            .arg(self.path("release"))
            .args(["--files"])
            .arg(root().join("LICENSE"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let archive = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
        let name = archive.file_name().unwrap().to_string_lossy().into_owned();
        let sums = Command::new("python3")
            .arg("-I")
            .arg("-c")
            .arg("import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())")
            .arg(&archive)
            .output()
            .unwrap();
        let digest = String::from_utf8(sums.stdout).unwrap().trim().to_owned();
        fs::write(
            self.path("release/SHA256SUMS"),
            format!("{digest}  {name}\n"),
        )
        .unwrap();
        name
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn script(name: &str) -> PathBuf {
    root().join("scripts/action").join(name)
}

fn run(sandbox: &Sandbox, name: &str, env: &[(&str, &str)], with_gh: bool) -> Output {
    let mut command = Command::new("bash");
    command
        .arg(script(name))
        .env_clear()
        .env("PATH", sandbox.restricted_path(with_gh))
        .env("HOME", sandbox.path("home"))
        .env("RUNNER_TEMP", sandbox.path("tmp"))
        .env("GITHUB_PATH", sandbox.path("github_path"))
        .env("GITHUB_OUTPUT", sandbox.path("github_output"));
    fs::create_dir_all(sandbox.path("tmp")).unwrap();
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn fetch_env(sandbox: &Sandbox, archive: &str) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("VERSION", "0.1.0".to_owned()),
        ("TARGET", "x86_64-unknown-linux-gnu".to_owned()),
        ("ARCHIVE", archive.to_owned()),
        ("EXE", String::new()),
        ("INSTALL_DIR", sandbox.path("install").display().to_string()),
        ("CACHE_DIR", sandbox.path("cache").display().to_string()),
    ])
}

fn fetch(
    sandbox: &Sandbox,
    env: &BTreeMap<&'static str, String>,
    extra: &[(&str, &str)],
) -> Output {
    let mut all: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    all.extend_from_slice(extra);
    run(sandbox, "fetch-and-verify.sh", &all, false)
}

#[test]
fn the_runner_maps_to_the_documented_targets() {
    let sandbox = Sandbox::new("target");
    let cases = [
        (
            "Linux",
            "X64",
            "gnu",
            "x86_64-unknown-linux-gnu",
            "libtea-1.2.3-x86_64-unknown-linux-gnu.tar.gz",
        ),
        (
            "Linux",
            "X64",
            "musl",
            "x86_64-unknown-linux-musl",
            "libtea-1.2.3-x86_64-unknown-linux-musl.tar.gz",
        ),
        (
            "Linux",
            "ARM64",
            "gnu",
            "aarch64-unknown-linux-gnu",
            "libtea-1.2.3-aarch64-unknown-linux-gnu.tar.gz",
        ),
        (
            "macOS",
            "ARM64",
            "gnu",
            "aarch64-apple-darwin",
            "libtea-1.2.3-aarch64-apple-darwin.tar.gz",
        ),
        (
            "macOS",
            "X64",
            "gnu",
            "x86_64-apple-darwin",
            "libtea-1.2.3-x86_64-apple-darwin.tar.gz",
        ),
        (
            "Windows",
            "X64",
            "gnu",
            "x86_64-pc-windows-msvc",
            "libtea-1.2.3-x86_64-pc-windows-msvc.zip",
        ),
    ];
    for (os, arch, libc, target, archive) in cases {
        let out = run(
            &sandbox,
            "resolve-target.sh",
            &[
                ("VERSION", "1.2.3"),
                ("RUNNER_OS", os),
                ("RUNNER_ARCH", arch),
                ("LIBC", libc),
            ],
            false,
        );
        assert!(out.status.success(), "{os}/{arch}: {}", text(&out.stderr));
        let stdout = text(&out.stdout);
        assert!(stdout.contains(&format!("target={target}\n")), "{stdout}");
        assert!(stdout.contains(&format!("archive={archive}\n")), "{stdout}");
    }
    for (os, arch, libc, version) in [
        ("Linux", "X86", "gnu", "1.2.3"),
        ("FreeBSD", "X64", "gnu", "1.2.3"),
        ("macOS", "ARM64", "musl", "1.2.3"),
        ("Linux", "X64", "uclibc", "1.2.3"),
        ("Linux", "X64", "gnu", "v1.2.3"),
        ("Linux", "X64", "gnu", "1.2.3;id"),
        ("Linux", "X64", "gnu", ""),
    ] {
        let out = run(
            &sandbox,
            "resolve-target.sh",
            &[
                ("VERSION", version),
                ("RUNNER_OS", os),
                ("RUNNER_ARCH", arch),
                ("LIBC", libc),
            ],
            false,
        );
        assert!(
            !out.status.success(),
            "{os}/{arch}/{libc}/{version:?} must be refused"
        );
    }
}

#[test]
fn a_matching_release_is_installed_cached_and_fetched_without_credentials() {
    let sandbox = Sandbox::new("ok");
    sandbox.install_curl();
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    let env = fetch_env(&sandbox, &archive);
    let out = fetch(
        &sandbox,
        &env,
        &[
            ("TEA_API_KEY_ID", "id-xyz"),
            ("TEA_API_KEY_SECRET", "leak-me"),
        ],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    let bin = sandbox.path("install/libtea");
    assert!(bin.exists());
    assert_eq!(
        fs::metadata(&bin).unwrap().permissions().mode() & 0o111,
        0o111
    );
    assert!(sandbox.path("cache").join(&archive).exists());
    assert!(sandbox.log("github_path").contains("install"));
    assert!(sandbox.log("github_output").contains("archive-path="));

    let log = sandbox.log("curl.log");
    for line in log.lines().filter(|l| l.starts_with("URL ")) {
        assert!(
            line.starts_with(
                "URL https://github.com/Mehrn0ush/libtea-rs/releases/download/v0.1.0/"
            ),
            "{line}"
        );
    }
    assert!(
        log.contains("FLAG --proto") && log.contains("FLAG =https") && log.contains("FLAG --fail")
    );
    assert!(
        log.contains("CRED unset/unset"),
        "credentials reached the downloader: {log}"
    );
    assert!(!text(&out.stdout).contains("leak-me") && !text(&out.stderr).contains("leak-me"));
}

#[test]
fn a_cached_archive_is_reused_only_when_it_matches_the_fresh_sums() {
    let sandbox = Sandbox::new("cache");
    sandbox.install_curl();
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    let env = fetch_env(&sandbox, &archive);
    assert!(fetch(&sandbox, &env, &[]).status.success());

    // Second run: SHA256SUMS is fetched again, the archive is not.
    fs::remove_file(sandbox.path("curl.log")).unwrap();
    assert!(fetch(&sandbox, &env, &[]).status.success());
    let log = sandbox.log("curl.log");
    assert!(log.contains("SHA256SUMS"), "{log}");
    assert!(
        !log.contains(&archive),
        "a verified cache hit must not download again: {log}"
    );

    // A poisoned cache entry is discarded and replaced.
    fs::write(sandbox.path("cache").join(&archive), b"poisoned").unwrap();
    fs::remove_file(sandbox.path("curl.log")).unwrap();
    let out = fetch(&sandbox, &env, &[]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert!(sandbox.log("curl.log").contains(&archive));
    assert!(text(&out.stderr).contains("does not match SHA256SUMS"));
    assert_ne!(
        fs::read(sandbox.path("cache").join(&archive)).unwrap(),
        b"poisoned"
    );
}

#[test]
fn a_tampered_archive_installs_nothing() {
    let sandbox = Sandbox::new("tamper");
    sandbox.install_curl();
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    // The release serves different bytes than SHA256SUMS describes.
    let mut bytes = fs::read(sandbox.path("release").join(&archive)).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(sandbox.path("release").join(&archive), bytes).unwrap();
    let env = fetch_env(&sandbox, &archive);
    let out = fetch(&sandbox, &env, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("does not match SHA256SUMS"),
        "{}",
        text(&out.stderr)
    );
    assert!(!sandbox.path("install/libtea").exists());
    assert!(
        !sandbox.path("cache").join(&archive).exists(),
        "a bad download must not be cached"
    );
}

#[test]
fn missing_duplicate_or_malformed_sums_are_refused() {
    let sandbox = Sandbox::new("sums");
    sandbox.install_curl();
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    let good = sandbox.log("release/SHA256SUMS");
    let env = fetch_env(&sandbox, &archive);
    for (label, body) in [
        (
            "no entry",
            "0000000000000000000000000000000000000000000000000000000000000000  other.tar.gz\n"
                .to_owned(),
        ),
        ("duplicate", format!("{good}{good}")),
        ("short digest", format!("abc  {archive}\n")),
        (
            "uppercase digest",
            good.to_uppercase()
                .replace(&archive.to_uppercase(), &archive),
        ),
        ("empty", String::new()),
    ] {
        fs::write(sandbox.path("release/SHA256SUMS"), body).unwrap();
        let out = fetch(&sandbox, &env, &[]);
        assert_eq!(out.status.code(), Some(1), "{label}: {}", text(&out.stderr));
        assert!(!sandbox.path("install/libtea").exists(), "{label}");
    }
    // A release with no SHA256SUMS at all.
    fs::remove_file(sandbox.path("release/SHA256SUMS")).unwrap();
    assert_eq!(fetch(&sandbox, &env, &[]).status.code(), Some(1));
}

#[test]
fn hostile_inputs_are_refused_before_any_download() {
    let sandbox = Sandbox::new("inputs");
    sandbox.install_curl();
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    let env = fetch_env(&sandbox, &archive);
    let cases: Vec<(&str, &str, &str)> = vec![
        ("VERSION", "0.1.0;id", "version"),
        ("VERSION", "v0.1.0", "version"),
        ("REPO", "owner/name;rm", "repo"),
        ("REPO", "../x", "repo"),
        ("REPO", "owner/..", "repo"),
        ("REPO", "./x", "repo"),
        ("REPO", "justname", "repo"),
        ("ARCHIVE", "../../etc/passwd", "archive"),
        ("ARCHIVE", "libtea-0.1.0-x.tar.gz/../y", "archive"),
        ("TARGET", "x86_64 unknown", "target"),
        ("EXE", ".sh", "exe"),
        ("RELEASE_BASE_URL", "http://example.com/x", "https"),
    ];
    for (key, value, what) in cases {
        let out = fetch(&sandbox, &env, &[(key, value)]);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{key}={value}: {}",
            text(&out.stderr)
        );
        assert!(
            !sandbox.log("curl.log").contains("URL"),
            "{what}: downloaded despite a bad {key}"
        );
    }
}

fn archive_for_attestation(sandbox: &Sandbox) -> String {
    let archive = sandbox.make_release("0.1.0", "x86_64-unknown-linux-gnu");
    sandbox.path("release").join(archive).display().to_string()
}

#[test]
fn the_attestation_is_verified_against_this_repository_and_workflow() {
    let sandbox = Sandbox::new("attest");
    let archive = archive_for_attestation(&sandbox);

    sandbox.install_gh(0);
    let out = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive)],
        true,
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    let call = sandbox.log("gh.log");
    assert!(call.contains("attestation verify"), "{call}");
    assert!(call.contains("--repo Mehrn0ush/libtea-rs"), "{call}");
    assert!(
        call.contains("--signer-workflow Mehrn0ush/libtea-rs/.github/workflows/release.yml"),
        "{call}"
    );
    assert!(sandbox
        .log("github_output")
        .contains("attestation=verified"));

    // Failure fails the step, in auto and in required mode.
    sandbox.install_gh(1);
    for mode in ["auto", "required"] {
        let out = run(
            &sandbox,
            "verify-attestation.sh",
            &[("ARCHIVE_PATH", &archive), ("ATTESTATION", mode)],
            true,
        );
        assert_eq!(out.status.code(), Some(1), "{mode}");
    }
    // A different repository is passed through (and validated).
    let other = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive), ("REPO", "a/b;c")],
        true,
    );
    assert_eq!(other.status.code(), Some(1));
}

#[test]
fn a_missing_gh_warns_unless_the_attestation_is_required_and_skip_does_not_call_it() {
    let sandbox = Sandbox::new("nogh");
    let archive = archive_for_attestation(&sandbox);
    let auto = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive)],
        false,
    );
    assert!(auto.status.success(), "{}", text(&auto.stderr));
    assert!(
        text(&auto.stderr).contains("not verified"),
        "{}",
        text(&auto.stderr)
    );
    assert!(sandbox
        .log("github_output")
        .contains("attestation=not-checked"));

    let required = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive), ("ATTESTATION", "required")],
        false,
    );
    assert_eq!(required.status.code(), Some(1));

    sandbox.install_gh(1);
    let skip = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive), ("ATTESTATION", "skip")],
        true,
    );
    assert!(skip.status.success());
    assert!(sandbox.log("gh.log").is_empty(), "skip must not call gh");

    let bad_mode = run(
        &sandbox,
        "verify-attestation.sh",
        &[("ARCHIVE_PATH", &archive), ("ATTESTATION", "maybe")],
        false,
    );
    assert_eq!(bad_mode.status.code(), Some(1));
}

#[test]
fn extraction_takes_one_regular_file_and_refuses_everything_else() {
    let sandbox = Sandbox::new("extract");
    let make = r#"
import io, sys, tarfile
out = sys.argv[1]
with tarfile.open(out, "w:gz") as t:
    def add(name, data=b"x", kind=tarfile.REGTYPE, link=""):
        info = tarfile.TarInfo(name); info.type = kind; info.linkname = link
        info.size = len(data) if kind == tarfile.REGTYPE else 0
        t.addfile(info, io.BytesIO(data) if kind == tarfile.REGTYPE else None)
    add("pkg/libtea", b"binary")
    add("pkg/link", kind=tarfile.SYMTYPE, link="/etc/passwd")
    add("pkg/hard", kind=tarfile.LNKTYPE, link="pkg/libtea")
    add("../escape", b"bad")
    add("pkg/big", b"0123456789")
"#;
    let archive = sandbox.path("a.tar.gz");
    let status = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(make)
        .arg(&archive)
        .status()
        .unwrap();
    assert!(status.success());
    let extract = |member: &str, dest: &str, extra: &[&str]| {
        Command::new("python3")
            .arg("-I")
            .arg(script("extract-member.py"))
            .arg(&archive)
            .arg(member)
            .arg(sandbox.path(dest))
            .args(extra)
            .output()
            .unwrap()
    };
    assert!(extract("pkg/libtea", "out1", &[]).status.success());
    assert_eq!(fs::read(sandbox.path("out1")).unwrap(), b"binary");
    for (member, extra) in [
        ("pkg/link", vec![]),
        ("pkg/hard", vec![]),
        ("../escape", vec![]),
        ("/abs", vec![]),
        ("pkg/missing", vec![]),
        ("pkg/big", vec!["--max-bytes", "5"]),
    ] {
        let out = extract(member, "out2", &extra);
        assert_eq!(out.status.code(), Some(1), "{member}");
        assert!(
            !sandbox.path("out2").exists(),
            "{member} left output behind"
        );
        assert!(
            !sandbox.path("out2.part").exists(),
            "{member} left a part file"
        );
    }
}

fn libtea_run(sandbox: &Sandbox, env: &[(&str, &str)]) -> Output {
    run(sandbox, "run-libtea.sh", env, false)
}

fn args_printed(out: &Output) -> Vec<String> {
    text(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("ARG:").map(str::to_owned))
        .collect()
}

#[test]
fn arguments_are_built_as_an_array_and_never_interpreted_by_a_shell() {
    let sandbox = Sandbox::new("args");
    sandbox.install_libtea(0);
    let out = libtea_run(
        &sandbox,
        &[
            ("COMMAND", "sync"),
            ("TEI", "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d"),
            ("LOCK_FILE", "tea.lock"),
            ("CHECK", "true"),
            (
                "EXTRA_ARGS",
                "--json\n\n--allow-origin\nhttp://127.0.0.1:1\r\n$(id)\n`id`\n; rm -rf /\n*\n--x=a b",
            ),
        ],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        args_printed(&out),
        [
            "sync",
            "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
            "--lock",
            "tea.lock",
            "--check",
            "--json",
            "--allow-origin",
            "http://127.0.0.1:1",
            "$(id)",
            "`id`",
            "; rm -rf /",
            "*",
            "--x=a b",
        ]
    );
    // Only the command name is echoed by the script itself.
    assert!(text(&out.stdout).contains("running: libtea sync\n"));
    assert!(sandbox.log("github_output").contains("exit-code=0"));
}

#[test]
fn each_command_gets_its_documented_arguments() {
    let sandbox = Sandbox::new("cmds");
    sandbox.install_libtea(0);
    let tei = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
    type Case<'a> = (Vec<(&'a str, &'a str)>, Vec<&'a str>);
    let cases: Vec<Case> = vec![
        (
            vec![("COMMAND", "sync"), ("TEI", tei), ("LOCK_FILE", "l")],
            vec!["sync", tei, "--lock", "l"],
        ),
        (
            vec![
                ("COMMAND", "diff"),
                ("LOCK_FILE", "a"),
                ("LOCK_FILE_NEW", "b"),
            ],
            vec!["diff", "a", "b"],
        ),
        (
            vec![("COMMAND", "discover"), ("TEI", tei)],
            vec!["discover", tei],
        ),
        (
            vec![("COMMAND", "parse-tei"), ("TEI", tei)],
            vec!["parse-tei", tei],
        ),
        (
            vec![
                ("COMMAND", "download"),
                ("BASE_URL", "https://api.example.com"),
            ],
            vec!["download", "--base", "https://api.example.com"],
        ),
        (vec![("COMMAND", "spec")], vec!["spec"]),
        (vec![("COMMAND", "conformance")], vec!["conformance"]),
    ];
    for (env, want) in cases {
        let out = libtea_run(&sandbox, &env);
        assert!(out.status.success(), "{env:?}: {}", text(&out.stderr));
        assert_eq!(args_printed(&out), want, "{env:?}");
    }
}

#[test]
fn bad_inputs_are_a_usage_error_and_credentials_cannot_ride_in_arguments() {
    let sandbox = Sandbox::new("bad");
    sandbox.install_libtea(0);
    let tei = "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d";
    let cases: Vec<Vec<(&str, &str)>> = vec![
        vec![],
        vec![("COMMAND", "rm")],
        vec![("COMMAND", "sync")],
        vec![("COMMAND", "sync"), ("TEI", tei)],
        vec![
            ("COMMAND", "sync"),
            ("TEI", "https://x"),
            ("LOCK_FILE", "l"),
        ],
        vec![("COMMAND", "sync"), ("TEI", tei), ("LOCK_FILE", "-rf")],
        vec![
            ("COMMAND", "sync"),
            ("TEI", tei),
            ("LOCK_FILE", "l"),
            ("CHECK", "yes"),
        ],
        vec![("COMMAND", "diff"), ("LOCK_FILE", "a")],
        vec![("COMMAND", "download")],
        vec![("COMMAND", "download"), ("BASE_URL", "ftp://x")],
        vec![
            ("COMMAND", "spec"),
            ("EXTRA_ARGS", "--api-key-secret=hunter2"),
        ],
        vec![
            ("COMMAND", "spec"),
            ("EXTRA_ARGS", "--json\n--api-key-secret\nhunter2"),
        ],
        vec![("COMMAND", "spec"), ("EXTRA_ARGS", "--api-key-id=me")],
        vec![("COMMAND", "spec"), ("WORKDIR", "-x")],
        vec![("COMMAND", "spec"), ("WORKDIR", "/no/such/dir")],
    ];
    for env in cases {
        let out = libtea_run(&sandbox, &env);
        assert_eq!(out.status.code(), Some(2), "{env:?}: {}", text(&out.stderr));
        assert!(
            out.stdout.is_empty(),
            "libtea must not have run for {env:?}"
        );
        assert!(
            !text(&out.stderr).contains("hunter2"),
            "a refused secret was echoed"
        );
    }
}

#[test]
fn libtea_sees_the_credentials_and_its_exit_status_is_propagated() {
    let sandbox = Sandbox::new("status");
    sandbox.install_libtea(7);
    let out = libtea_run(
        &sandbox,
        &[
            ("COMMAND", "spec"),
            ("TEA_API_KEY_ID", "the-id"),
            ("TEA_API_KEY_SECRET", "the-secret-value"),
            ("WORKDIR", "/"),
        ],
    );
    assert_eq!(out.status.code(), Some(7));
    assert!(sandbox.log("github_output").contains("exit-code=7"));
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("KEYSET:yes"),
        "the binary must receive the environment credentials"
    );
    assert!(stdout.contains("PWD:/"), "{stdout}");
    // The script itself never prints a credential.
    assert!(!stdout.replace("KEYSET:yes", "").contains("the-id"));
    assert!(
        !stdout.contains("the-secret-value") && !text(&out.stderr).contains("the-secret-value")
    );
}

#[test]
fn a_missing_binary_is_reported_and_the_scripts_pass_shellcheck() {
    let sandbox = Sandbox::new("nobin");
    let out = libtea_run(&sandbox, &[("COMMAND", "spec")]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("not on PATH"));

    if let Some(shellcheck) = have("shellcheck") {
        let mut command = Command::new(shellcheck);
        // Run from the repository root so the `source=` directives resolve.
        command.current_dir(root()).arg("--shell=bash").arg("-x");
        for name in [
            "common.sh",
            "resolve-target.sh",
            "fetch-and-verify.sh",
            "verify-attestation.sh",
            "run-libtea.sh",
        ] {
            command.arg(format!("scripts/action/{name}"));
        }
        let result = command.output().unwrap();
        assert!(result.status.success(), "{}", text(&result.stdout));
    }
}
