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
            "api-key-id",
            "api-key-secret",
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
fn credentials_are_inputs_that_only_the_final_step_reads() {
    let text = action();
    // Only these two inputs may look like credentials.
    for name in input_names(&text) {
        let lower = name.to_ascii_lowercase();
        let credential_like = ["secret", "token", "key", "password", "credential", "auth"]
            .iter()
            .any(|word| lower.contains(word));
        assert_eq!(
            credential_like,
            matches!(name.as_str(), "api-key-id" | "api-key-secret"),
            "input `{name}`"
        );
    }
    // The action never reads repository secrets itself.
    for line in code_lines(&text) {
        assert!(!line.contains("secrets."), "{line}");
    }
    let all = steps(&text);
    let run = all
        .iter()
        .find(|s| s.name == "Run libtea")
        .expect("a run step");
    assert_eq!(
        all.last().unwrap().name,
        "Run libtea",
        "the run step is last"
    );
    // Each credential input is referenced exactly once, in the run step's env.
    for (input, var) in [
        ("inputs.api-key-id", "TEA_API_KEY_ID"),
        ("inputs.api-key-secret", "TEA_API_KEY_SECRET"),
    ] {
        let code = code_lines(&text).join("\n");
        assert_eq!(
            code.matches(input).count(),
            1,
            "{input} must be used exactly once"
        );
        assert!(
            run.raw.contains(&format!("{var}: ${{{{ {input} }}}}")),
            "{input} must feed {var} in the run step:\n{}",
            run.raw
        );
    }
    // No other step mentions a credential variable or the GitHub token.
    for step in &all {
        if step.name != "Run libtea" {
            assert!(
                !step.raw.contains("TEA_API_KEY")
                    || step.name == "Refuse credentials in the ambient environment",
                "step `{}` handles credentials",
                step.name
            );
        }
    }
    assert!(!run.raw.contains("GH_TOKEN") && !run.raw.contains("github.token"));
}

#[test]
fn the_ambient_credential_guard_runs_before_every_other_step() {
    let text = action();
    let all = steps(&text);
    assert_eq!(all[0].name, "Refuse credentials in the ambient environment");
    assert!(all[0]
        .raw
        .contains("scripts/action/refuse-ambient-credentials.sh"));
    // In particular before any third-party action runs.
    let first_uses = all.iter().position(|s| s.raw.contains("uses: ")).unwrap();
    assert!(first_uses > 0);
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
    assert_eq!(all.len(), 6);
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
            let (name, rest) = uses.split_once('@').unwrap();
            assert_eq!(name, "actions/cache");
            // `<40-hex sha> # vX.Y.Z`: the comment says which release the SHA is.
            let (reference, comment) = match rest.split_once(" # ") {
                Some((reference, comment)) => (reference, Some(comment)),
                None => (rest, None),
            };
            let sha = reference.len() == 40 && reference.bytes().all(|b| b.is_ascii_hexdigit());
            assert!(sha || reference == "PIN_ME_ACTIONS_CACHE", "{reference}");
            if sha {
                assert!(
                    comment.is_some_and(|c| c.starts_with('v')),
                    "a pinned SHA needs a `# vX.Y.Z` comment: {uses}"
                );
            }
        }
    }
    assert_eq!(seen, 1);
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
                    matches!(key, "api-key-id" | "api-key-secret"),
                    "a secret is used outside the two credential inputs: {line}"
                );
                assert_eq!(
                    indent(line),
                    10,
                    "credentials belong directly under the libtea step's with: {line}"
                );
            }
            // The ambient-environment route is refused by the action itself.
            assert!(
                !line.trim_start().starts_with("TEA_API_KEY"),
                "examples must not use env for credentials: {line}"
            );
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
                assert!(
                    !key.contains(word) || matches!(key.as_str(), "api-key-id" | "api-key-secret"),
                    "suspicious input `{key}`"
                );
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

#[test]
fn the_action_has_what_a_marketplace_listing_needs() {
    let text = action();
    let lines = code_lines(&text);
    let top = |key: &str| {
        lines
            .iter()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim().trim_matches('"').to_owned())
    };
    assert_eq!(top("name:").as_deref(), Some("libtea-rs"));
    let description = top("description:").expect("a top-level description");
    assert!(
        !description.is_empty() && description.len() <= 125,
        "the Marketplace shows at most 125 characters of the description: {description:?}"
    );

    let at = lines
        .iter()
        .position(|l| *l == "branding:")
        .expect("a top-level branding: block");
    let block: Vec<&str> = lines[at + 1..]
        .iter()
        .take_while(|l| indent(l) > 0)
        .copied()
        .collect();
    let field = |key: &str| {
        block
            .iter()
            .find_map(|l| l.trim().strip_prefix(key))
            .map(|v| v.trim().to_owned())
    };
    let icon = field("icon:").expect("branding.icon");
    assert!(
        !icon.is_empty() && icon.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
        "branding.icon must be a Feather icon name, got {icon:?}"
    );
    let color = field("color:").expect("branding.color");
    assert!(
        [
            "white",
            "black",
            "yellow",
            "blue",
            "green",
            "orange",
            "red",
            "purple",
            "gray-dark"
        ]
        .contains(&color.as_str()),
        "branding.color must be one GitHub accepts, got {color:?}"
    );
}
