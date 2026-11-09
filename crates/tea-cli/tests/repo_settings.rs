//! `scripts/check-repo-settings.sh` and `.github/workflows/repo-settings.yml`:
//! the check that reports missing branch and tag rulesets. `gh` is replaced by a
//! stub that serves fixtures, so nothing here talks to GitHub.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

const CHECKS: [&str; 6] = [
    "rust stable",
    "rust 1.89.0",
    "package",
    "cargo deny",
    "cargo semver-checks",
    "non_exhaustive guards",
];

fn branch_rules(types: &[&str], checks: &[&str]) -> Value {
    let rules: Vec<Value> = types
        .iter()
        .map(|t| {
            if *t == "required_status_checks" {
                let contexts: Vec<Value> = checks.iter().map(|c| json!({"context": c})).collect();
                json!({"type": t, "parameters": {"required_status_checks": contexts}})
            } else {
                json!({"type": t})
            }
        })
        .collect();
    Value::Array(rules)
}

fn tag_ruleset(enforcement: &str, include: &str, types: &[&str]) -> Value {
    json!({
        "id": 7,
        "target": "tag",
        "enforcement": enforcement,
        "conditions": {"ref_name": {"include": [include], "exclude": []}},
        "rules": types.iter().map(|t| json!({"type": t})).collect::<Vec<_>>()
    })
}

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str, branch: Option<Value>, tag_ruleset: Option<Value>) -> Self {
        Self::with_actions(tag, branch, tag_ruleset, None)
    }

    fn with_actions(
        tag: &str,
        branch: Option<Value>,
        tag_ruleset: Option<Value>,
        actions: Option<Value>,
    ) -> Self {
        let dir =
            std::env::temp_dir().join(format!("libtea-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let data = |name: &str, value: &Value| {
            std::fs::write(dir.join("data").join(name), value.to_string()).unwrap();
        };
        if let Some(branch) = branch {
            data("repos_o_r_rules_branches_main", &branch);
        }
        if let Some(actions) = actions {
            data("repos_o_r_actions_permissions", &actions);
        }
        let mut list = Vec::new();
        if let Some(ruleset) = tag_ruleset {
            list.push(json!({"id": 7, "target": "tag", "enforcement": ruleset["enforcement"]}));
            data("repos_o_r_rulesets_7", &ruleset);
        }
        data("repos_o_r_rulesets", &Value::Array(list));
        // `gh api <path>`: serve data/<path with / as _>, or fail like gh does.
        let gh = dir.join("bin/gh");
        std::fs::write(
            &gh,
            format!(
                "#!/bin/sh\n[ \"$1\" = api ] || exit 2\nf=\"{}/data/$(printf %s \"$2\" | tr / _)\"\n[ -f \"$f\" ] || {{ echo 'HTTP 404' >&2; exit 1; }}\ncat \"$f\"\n",
                dir.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }

    fn run(&self) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new("bash")
            .arg(root().join("scripts/check-repo-settings.sh"))
            .args(["o/r", "main"])
            .env("PATH", path)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

const GOOD_BRANCH: [&str; 4] = [
    "pull_request",
    "required_status_checks",
    "non_fast_forward",
    "deletion",
];
const GOOD_TAG: [&str; 4] = ["creation", "update", "deletion", "non_fast_forward"];

#[test]
fn complete_settings_pass() {
    if !python_available() {
        return;
    }
    let f = Fixture::new(
        "ok",
        Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
        Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
    );
    let out = f.run();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("repository settings: ok"));
}

#[test]
fn each_missing_setting_is_reported() {
    if !python_available() {
        return;
    }
    let cases: Vec<(&str, Option<Value>, Option<Value>, &str)> = vec![
        (
            "nopr",
            Some(branch_rules(
                &["required_status_checks", "non_fast_forward", "deletion"],
                &CHECKS,
            )),
            Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
            "does not require a pull request",
        ),
        (
            "nocheck",
            Some(branch_rules(&["pull_request", "non_fast_forward", "deletion"], &[])),
            Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
            "does not require status checks",
        ),
        (
            "somechecks",
            Some(branch_rules(&GOOD_BRANCH, &["rust stable", "package"])),
            Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
            "required status checks are missing: cargo deny, cargo semver-checks, non_exhaustive guards, rust 1.89.0",
        ),
        (
            "noforce",
            Some(branch_rules(
                &["pull_request", "required_status_checks", "deletion"],
                &CHECKS,
            )),
            Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
            "force pushes to main are not blocked",
        ),
        (
            "norules",
            Some(json!([])),
            Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
            "no rules apply to main",
        ),
        (
            "notag",
            Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
            None,
            "no active tag ruleset covers v*",
        ),
        (
            "tagdisabled",
            Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
            Some(tag_ruleset("disabled", "refs/tags/v*", &GOOD_TAG)),
            "no active tag ruleset covers v*",
        ),
        (
            "tagwrongpattern",
            Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
            Some(tag_ruleset("active", "refs/tags/release-*", &GOOD_TAG)),
            "no active tag ruleset covers v*",
        ),
        (
            "tagweak",
            Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
            Some(tag_ruleset("active", "refs/tags/v*", &["creation", "update"])),
            "no active tag ruleset covers v*",
        ),
    ];
    for (tag, branch, tag_rules, expected) in cases {
        let out = Fixture::new(tag, branch, tag_rules).run();
        assert_eq!(out.status.code(), Some(1), "{tag}: {}", stderr(&out));
        assert!(stderr(&out).contains(expected), "{tag}: {}", stderr(&out));
        assert!(
            !String::from_utf8_lossy(&out.stdout).contains("ok"),
            "{tag}"
        );
    }
}

#[test]
fn an_unreadable_repository_is_a_setup_error_not_a_pass() {
    if !python_available() {
        return;
    }
    // No branch fixture: the stub answers like a 404 (a private repository on a
    // plan without rulesets, or a token that may not read them).
    let f = Fixture::new("unreadable", None, None);
    let out = f.run();
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("cannot read the rules"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn the_required_checks_are_the_jobs_ci_defines() {
    let script = std::fs::read_to_string(root().join("scripts/check-repo-settings.sh")).unwrap();
    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let line = script
        .lines()
        .find(|l| l.starts_with("REQUIRED_CHECKS="))
        .expect("REQUIRED_CHECKS");
    let listed: Vec<&str> = line
        .trim_start_matches("REQUIRED_CHECKS=")
        .trim_matches('"')
        .split('|')
        .collect();
    assert_eq!(
        listed, CHECKS,
        "the test and the script list the same checks"
    );
    for check in listed {
        if let Some(version) = check.strip_prefix("rust ") {
            assert!(ci.contains("name: rust ${{ matrix.rust }}"));
            assert!(ci.contains(&format!("\"{version}\"")), "{check}");
        } else {
            assert!(ci.contains(&format!("name: {check}\n")), "{check}");
        }
    }
}

#[test]
fn the_workflow_only_reads_and_uses_the_token_through_env() {
    let text = std::fs::read_to_string(root().join(".github/workflows/repo-settings.yml")).unwrap();
    assert!(text.contains("permissions:\n  contents: read\n"));
    assert!(!text.contains(": write"), "no write permission");
    assert!(!text.contains("pull_request"), "no pull-request trigger");
    assert!(text.contains("persist-credentials: false"));
    let uses: Vec<&str> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("uses: "))
        .collect();
    assert_eq!(uses.len(), 1, "{uses:?}");
    let (action, sha) = uses[0].split_once('@').unwrap();
    // A pinned SHA is followed by a `# vX.Y.Z` comment.
    let sha = sha.split(" # ").next().unwrap();
    assert_eq!(action, "actions/checkout");
    assert!(
        sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()),
        "{sha}"
    );
    // The token and the repository name reach the script as env, never inside the
    // shell text.
    for line in text.lines().filter(|l| l.trim().starts_with("run:")) {
        assert!(!line.contains("${{"), "{line}");
    }
    assert!(text.contains("GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}"));
}

fn good(tag: &str, actions: Option<Value>) -> Fixture {
    Fixture::with_actions(
        tag,
        Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
        Some(tag_ruleset("active", "refs/tags/v*", &GOOD_TAG)),
        actions,
    )
}

#[test]
fn local_only_actions_is_a_problem_that_names_the_symptom() {
    if !python_available() {
        return;
    }
    let out = good(
        "local",
        Some(json!({"enabled": true, "allowed_actions": "local_only"})),
    )
    .run();
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("startup_failure"), "{}", stderr(&out));
    assert!(stderr(&out).contains("Allow all actions and reusable workflows"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("ok"));
}

#[test]
fn all_actions_pass_and_say_the_setting_was_checked() {
    if !python_available() {
        return;
    }
    let out = good(
        "allall",
        Some(json!({"enabled": true, "allowed_actions": "all"})),
    )
    .run();
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "repository settings: ok"
    );
    assert!(!stderr(&out).contains("cannot check"));
}

#[test]
fn unreadable_actions_permissions_are_reported_as_not_checked() {
    if !python_available() {
        return;
    }
    // No fixture: the stub answers HTTP 404, as a read-only token gets (403 or 404).
    let out = good("noadmin", None).run();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("cannot check the Actions permissions here"),
        "{}",
        stderr(&out)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        stdout.contains("Actions permissions not verified"),
        "{stdout}"
    );
    // Not a pass for that setting, and not a failure for the others: a real
    // problem elsewhere still fails the run.
    let broken = Fixture::new(
        "noadmin-broken",
        Some(branch_rules(&GOOD_BRANCH, &CHECKS)),
        None,
    )
    .run();
    assert_eq!(broken.status.code(), Some(1));
}
