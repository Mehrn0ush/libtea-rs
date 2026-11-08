//! Policy lint for `action.yml` and the example workflows in
//! `docs/github-action.md`: credentials never travel as inputs, inputs reach
//! scripts only through `env:`, and the examples are safe to copy.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn action() -> String {
    std::fs::read_to_string(root().join("action.yml")).unwrap()
}

fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

struct Step {
    name: String,
    raw: String,
}

/// Steps of the composite action (`    - name:` at four spaces).
fn steps(text: &str) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    let mut in_steps = false;
    for line in code_lines(text) {
        if line == "  steps:" {
            in_steps = true;
            continue;
        }
        if !in_steps {
            continue;
        }
        if let Some(name) = line.strip_prefix("    - name: ") {
            out.push(Step {
                name: name.trim().to_owned(),
                raw: String::new(),
            });
        } else if line.starts_with("    - ") {
            out.push(Step {
                name: String::new(),
                raw: String::new(),
            });
        }
        if let Some(step) = out.last_mut() {
            step.raw.push_str(line);
            step.raw.push('\n');
        }
    }
    out
}

fn run_lines(step: &Step) -> Vec<&str> {
    step.raw
        .lines()
        .filter(|l| l.trim_start().starts_with("run:"))
        .collect()
}

fn input_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_inputs = false;
    for line in code_lines(text) {
        if line == "inputs:" {
            in_inputs = true;
            continue;
        }
        if in_inputs {
            if !line.is_empty() && indent(line) == 0 {
                break;
            }
            if indent(line) == 2 && line.trim_end().ends_with(':') {
                names.push(line.trim().trim_end_matches(':').to_owned());
            }
        }
    }
    names
}

#[test]
fn the_action_is_composite_and_has_the_documented_inputs() {
    let text = action();
    assert!(text.contains("\n  using: composite\n"));
    assert_eq!(
        input_names(&text),
        [
            "version",
            "command",
            "tei",
            "lock-file",
            "lock-file-new",
            "base-url",
            "check",
            "extra-args",
            "working-directory",
            "libc",
            "repository",
            "attestation",
            "cache"
        ]
    );
}

#[test]
fn no_input_can_carry_a_credential() {
    let text = action();
    for name in input_names(&text) {
        let lower = name.to_ascii_lowercase();
        for word in ["secret", "token", "key", "password", "credential", "auth"] {
            assert!(
                !lower.contains(word),
                "input `{name}` looks like a credential"
            );
        }
    }
    for line in code_lines(&text) {
        assert!(
            !line.contains("secrets."),
            "the action must not read secrets: {line}"
        );
        assert!(
            !line.contains("TEA_API_KEY"),
            "the action must not map credentials: {line}"
        );
    }
}

#[test]
fn only_the_attestation_step_sees_the_github_token() {
    let text = action();
    for step in steps(&text) {
        let uses_token = step.raw.contains("github.token") || step.raw.contains("GH_TOKEN");
        assert_eq!(
            uses_token,
            step.name == "Verify the build-provenance attestation",
            "token use in step `{}`",
            step.name
        );
    }
}

#[test]
fn inputs_reach_scripts_through_env_only() {
    let text = action();
    let all = steps(&text);
    assert_eq!(all.len(), 5);
    for step in &all {
        for line in run_lines(step) {
            assert!(
                !line.contains("${{"),
                "expression in a run line of `{}`: {line}",
                step.name
            );
            assert!(
                line.contains("bash \"$ACTION_PATH/scripts/action/") && line.ends_with(".sh\""),
                "unexpected run line in `{}`: {line}",
                step.name
            );
        }
        if !run_lines(step).is_empty() {
            assert!(
                step.raw.contains("shell: bash"),
                "`{}` needs shell: bash",
                step.name
            );
            assert!(step.raw.contains("ACTION_PATH: ${{ github.action_path }}"));
        }
    }
    // Every declared input is used, and every use is a declared input.
    let declared: BTreeSet<String> = input_names(&text).into_iter().collect();
    let code = code_lines(&text).join("\n");
    let used: BTreeSet<String> = code
        .match_indices("inputs.")
        .map(|(at, _)| {
            code[at + 7..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect::<String>()
        })
        .collect();
    assert_eq!(declared, used);
}

#[test]
fn the_scripts_the_action_runs_exist_and_fail_closed() {
    let text = action();
    for name in [
        "resolve-target",
        "fetch-and-verify",
        "verify-attestation",
        "run-libtea",
    ] {
        assert!(
            text.contains(&format!("scripts/action/{name}.sh")),
            "{name} not wired"
        );
        let script =
            std::fs::read_to_string(root().join(format!("scripts/action/{name}.sh"))).unwrap();
        assert!(
            script.contains("set -euo pipefail") || script.contains("set -uo pipefail"),
            "{name}"
        );
    }
}

#[test]
fn the_only_third_party_action_is_cache_and_it_is_pinned_or_a_placeholder() {
    let text = action();
    let mut seen = 0;
    for line in code_lines(&text) {
        if let Some(uses) = line.trim().strip_prefix("uses: ") {
            seen += 1;
            let (name, reference) = uses.split_once('@').unwrap();
            assert_eq!(name, "actions/cache");
            let sha = reference.len() == 40 && reference.bytes().all(|b| b.is_ascii_hexdigit());
            assert!(sha || reference == "PIN_ME_ACTIONS_CACHE", "{reference}");
        }
    }
    assert_eq!(seen, 1);
}

/// Run before the first release: fails while the cache pin is a placeholder.
#[test]
#[ignore = "fails until docs/release-pins.md is followed; run with --ignored before tagging"]
fn action_has_no_unresolved_pins() {
    let text = action();
    assert!(!code_lines(&text).iter().any(|l| l.contains("PIN_ME_")));
}

// ---------------------------------------------------------------------------
// The examples in docs/github-action.md are copied into real workflows.
// ---------------------------------------------------------------------------

fn yaml_blocks() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("docs/github-action.md")).unwrap();
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.starts_with("```yaml") {
            current = Some(String::new());
        } else if line.starts_with("```") {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
}

#[test]
fn example_workflows_are_safe_to_copy() {
    let blocks = yaml_blocks();
    let workflows: Vec<&String> = blocks.iter().filter(|b| b.contains("\njobs:\n")).collect();
    assert!(
        workflows.len() >= 4,
        "found {} example workflows",
        workflows.len()
    );
    for workflow in &workflows {
        assert!(!workflow.contains("pull_request_target"), "{workflow}");
        assert!(
            workflow.contains("\npermissions:\n  contents: read\n"),
            "{workflow}"
        );
        assert!(!workflow.contains(": write"), "{workflow}");
        for line in workflow.lines() {
            if line.contains("secrets.") {
                let key = line.trim().split(':').next().unwrap();
                assert!(
                    matches!(key, "TEA_API_KEY_ID" | "TEA_API_KEY_SECRET"),
                    "a secret is used outside the two credential env vars: {line}"
                );
                assert!(
                    indent(line) >= 10,
                    "credentials belong in a step's env: {line}"
                );
            }
            if let Some(uses) = line.trim().strip_prefix("- uses: ") {
                let (_, reference) = uses.split_once('@').expect("uses: owner/repo@ref");
                assert!(
                    reference == "<commit-sha>",
                    "examples pin by commit SHA: {line}"
                );
            }
        }
        // Inputs of the action never carry a credential.
        for key in with_keys(workflow) {
            let key = key.to_ascii_lowercase();
            for word in ["secret", "token", "password", "api-key", "key-id"] {
                assert!(!key.contains(word), "suspicious input `{key}`");
            }
        }
        for line in workflow
            .lines()
            .filter(|l| l.trim_start().starts_with("- uses: actions/checkout"))
        {
            let at = workflow.find(line).unwrap();
            assert!(workflow[at..]
                .lines()
                .take(3)
                .any(|l| l.contains("persist-credentials: false")));
        }
    }
    // One of them is the `sync --check` job the milestone asks for.
    assert!(workflows
        .iter()
        .any(|w| w.contains("command: sync") && w.contains("check: \"true\"")));
}

/// Keys directly under a step's `with:` (step at indent 6, `with:` at 8).
fn with_keys(block: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut in_with = false;
    for line in block.lines() {
        if line.trim() == "with:" && indent(line) == 8 {
            in_with = true;
            continue;
        }
        if in_with {
            if line.trim().is_empty() || indent(line) < 10 {
                in_with = false;
                continue;
            }
            if indent(line) == 10 {
                keys.push(line.trim().split(':').next().unwrap().to_owned());
            }
        }
    }
    keys
}

#[test]
fn examples_use_only_documented_inputs() {
    let declared: BTreeSet<String> = input_names(&action()).into_iter().collect();
    let mut checked = 0;
    for block in yaml_blocks() {
        for key in with_keys(&block) {
            // `with:` also belongs to actions/checkout.
            if key != "persist-credentials" {
                assert!(
                    declared.contains(&key),
                    "unknown input `{key}` in an example"
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 12, "only {checked} inputs were checked");
}

#[test]
fn the_attestation_step_is_told_which_release_it_is_verifying() {
    let text = action();
    let all = steps(&text);
    let step = all
        .iter()
        .find(|s| s.name == "Verify the build-provenance attestation")
        .expect("an attestation step");
    assert!(
        step.raw.contains("VERSION: ${{ inputs.version }}"),
        "the version selects the tag ref the attestation must be for:\n{}",
        step.raw
    );
}

#[test]
fn the_installation_docs_pin_the_attestation_to_the_tag() {
    let docs = std::fs::read_to_string(root().join("docs/installing.md")).unwrap();
    let block = docs
        .split("gh attestation verify")
        .nth(1)
        .expect("installing.md shows gh attestation verify");
    let command: String = block.lines().take(5).collect::<Vec<_>>().join("\n");
    for flag in [
        "--repo",
        "--signer-workflow",
        "--source-ref \"refs/tags/v$VERSION\"",
    ] {
        assert!(command.contains(flag), "`{flag}` missing from:\n{command}");
    }
    // The checklist makes the settings that back the workflow explicit.
    let checklist = std::fs::read_to_string(root().join("docs/release-checklist.md")).unwrap();
    for needed in [
        "Environment `release`",
        "Tag ruleset",
        "Branch protection",
        "Private vulnerability reporting",
        "publish the release by hand",
    ] {
        assert!(
            checklist.contains(needed),
            "release-checklist.md lacks `{needed}`"
        );
    }
}
