//! The pin gate: `PIN_ME_*` placeholders in `release.yml` and `action.yml` block
//! a release. Tests run the gate on the real files, so `cargo test` fails while a
//! placeholder remains unless the documented development flag is set.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FLAG: &str = "LIBTEA_ALLOW_PIN_PLACEHOLDERS";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

/// Non-comment lines holding a placeholder, as `file:text`.
fn placeholders_in(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for file in [".github/workflows/release.yml", "action.yml"] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        for line in text.lines() {
            if line.contains("PIN_ME_") && !line.trim_start().starts_with('#') {
                found.push(format!("{file}: {}", line.trim()));
            }
        }
    }
    found
}

/// The gate: a placeholder blocks unless the flag is exactly `1`.
fn gate(found: &[String], flag: Option<&str>) -> Result<(), String> {
    if found.is_empty() || flag == Some("1") {
        return Ok(());
    }
    Err(format!(
        "unresolved pins ({} left); follow docs/release-pins.md, or for development only set {FLAG}=1:\n{}",
        found.len(),
        found.join("\n")
    ))
}

#[test]
fn the_gate_passes_only_without_placeholders_or_with_the_exact_flag() {
    let one = vec!["release.yml: uses: x@PIN_ME_X".to_owned()];
    assert!(gate(&[], None).is_ok());
    assert!(gate(&[], Some("0")).is_ok());
    assert!(gate(&one, Some("1")).is_ok());
    for flag in [
        None,
        Some(""),
        Some("0"),
        Some("true"),
        Some("yes"),
        Some("11"),
        Some(" 1"),
        Some("1 "),
    ] {
        assert!(gate(&one, flag).is_err(), "{flag:?} must not open the gate");
    }
}

#[test]
fn unresolved_pins_block_unless_the_documented_flag_is_set() {
    let found = placeholders_in(&root());
    let flag = std::env::var(FLAG).ok();
    if let Err(message) = gate(&found, flag.as_deref()) {
        panic!("{message}");
    }
}

#[test]
fn ci_sets_the_flag_exactly_while_placeholders_remain() {
    let ci = read(".github/workflows/ci.yml");
    let sets_flag = ci.lines().any(|l| l.trim() == format!("{FLAG}: \"1\""));
    let placeholders = !placeholders_in(&root()).is_empty();
    assert_eq!(
        sets_flag, placeholders,
        "ci.yml must set {FLAG}=1 while placeholders remain and must not once they are gone"
    );
    // Only the test job gets it, and nothing else in the repository sets it.
    assert!(!read(".github/workflows/release.yml").contains(FLAG));
    assert!(!read("action.yml").contains(FLAG));
}

fn check_pins(root: &Path, args: &[&str], flag: Option<&str>) -> Output {
    let mut command = Command::new("bash");
    command
        .arg(self::root().join("scripts/check-pins.sh"))
        .args(args)
        .env("LIBTEA_ROOT", root)
        .env_remove(FLAG);
    if let Some(flag) = flag {
        command.env(FLAG, flag);
    }
    command.output().unwrap()
}

fn temp_root(tag: &str, release: &str, action: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("libtea-pins-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(dir.join(".github/workflows/release.yml"), release).unwrap();
    std::fs::write(dir.join("action.yml"), action).unwrap();
    dir
}

#[test]
fn the_script_blocks_a_release_even_with_the_development_flag() {
    let with = temp_root(
        "with",
        "steps:\n  - uses: actions/upload-artifact@PIN_ME_UPLOAD_ARTIFACT\n",
        "steps: []\n",
    );
    let clean = temp_root(
        "clean",
        "steps:\n  # PIN_ME_ mentioned only in a comment\n  - uses: a/b@0123456789012345678901234567890123456789\n",
        "steps: []\n",
    );
    let in_action = temp_root(
        "act",
        "steps: []\n",
        "      uses: actions/cache@PIN_ME_ACTIONS_CACHE\n",
    );
    let in_env = temp_root(
        "env",
        "env:\n  CARGO_CYCLONEDX_VERSION: PIN_ME_CARGO_CYCLONEDX_VERSION\n",
        "steps: []\n",
    );

    for placeholder in [&with, &in_action, &in_env] {
        // Development: blocked without the flag, allowed with exactly 1.
        assert_eq!(check_pins(placeholder, &[], None).status.code(), Some(1));
        assert_eq!(
            check_pins(placeholder, &[], Some("true")).status.code(),
            Some(1)
        );
        let allowed = check_pins(placeholder, &[], Some("1"));
        assert_eq!(allowed.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&allowed.stderr).contains("development only"));
        // Release: the flag is never read.
        let blocked = check_pins(placeholder, &["--release"], Some("1"));
        assert_eq!(
            blocked.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&blocked.stderr)
        );
        assert!(String::from_utf8_lossy(&blocked.stderr).contains("PIN_ME_"));
    }
    // No placeholder (a comment does not count): fine in both modes.
    for args in [&[][..], &["--release"][..]] {
        assert_eq!(check_pins(&clean, args, None).status.code(), Some(0));
    }
    assert_eq!(
        check_pins(&clean, &["--bogus"], None).status.code(),
        Some(2)
    );
    for dir in [with, clean, in_action, in_env] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn the_release_workflow_runs_the_gate_first_and_never_sets_the_flag() {
    let text = read(".github/workflows/release.yml");
    let meta: Vec<&str> = text
        .lines()
        .skip_while(|l| *l != "  meta:")
        .take_while(|l| !l.starts_with("  build:"))
        .collect();
    let meta = meta.join("\n");
    let checkout = meta.find("actions/checkout@").expect("meta checks out");
    let gate_at = meta
        .find("./scripts/check-pins.sh --release")
        .expect("meta runs the gate");
    let tag_at = meta
        .find("check-release-tag.sh")
        .expect("meta checks the tag");
    assert!(
        checkout < gate_at && gate_at < tag_at,
        "the gate runs right after checkout"
    );
    assert!(!text.contains(FLAG));
}

/// The commands in `docs/release-pins.md` (the `sh pins-apply` block) must
/// really resolve every placeholder and retire the CI flag.
#[test]
fn the_documented_commands_resolve_every_placeholder() {
    let docs = read("docs/release-pins.md");
    let block = docs
        .split("```sh pins-apply\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("docs/release-pins.md has a `sh pins-apply` block");

    let dir = std::env::temp_dir().join(format!("libtea-pins-apply-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    for file in [
        ".github/workflows/release.yml",
        ".github/workflows/ci.yml",
        "action.yml",
        "scripts/check-pins.sh",
    ] {
        std::fs::copy(root().join(file), dir.join(file)).unwrap();
    }
    let script = format!(
        "set -euo pipefail\n\
         UPLOAD_SHA=1111111111111111111111111111111111111111\n\
         DOWNLOAD_SHA=2222222222222222222222222222222222222222\n\
         ATTEST_SHA=3333333333333333333333333333333333333333\n\
         CACHE_SHA=4444444444444444444444444444444444444444\n\
         CYCLONEDX=0.5.7\n{block}"
    );
    let applied = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .current_dir(&dir)
        .env_remove(FLAG)
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(
        placeholders_in(&dir).is_empty(),
        "{:?}",
        placeholders_in(&dir)
    );
    let gate = check_pins(&dir, &["--release"], None);
    assert_eq!(
        gate.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&gate.stderr)
    );
    let release = std::fs::read_to_string(dir.join(".github/workflows/release.yml")).unwrap();
    for sha in [
        "1111111111111111111111111111111111111111",
        "2222222222222222222222222222222222222222",
        "3333333333333333333333333333333333333333",
    ] {
        assert!(release.contains(&format!("@{sha}")), "{sha}");
    }
    assert!(release.contains("CARGO_CYCLONEDX_VERSION: 0.5.7"));
    assert!(std::fs::read_to_string(dir.join("action.yml"))
        .unwrap()
        .contains("@4444444444444444444444444444444444444444"));
    assert!(
        !std::fs::read_to_string(dir.join(".github/workflows/ci.yml"))
            .unwrap()
            .contains(FLAG),
        "the CI flag must be retired together with the placeholders"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
