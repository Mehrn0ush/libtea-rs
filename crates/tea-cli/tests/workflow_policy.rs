//! Policy lint for `.github/workflows/interop-manual.yml`.
//!
//! The workflow hands repository secrets to the probe, so the target URL must
//! come from the protected Environment, not from whoever dispatches it.

use std::path::Path;

fn workflow() -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/interop-manual.yml");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn interop_workflow_is_dispatch_only_and_environment_scoped() {
    let text = workflow();
    let on_block: String = text
        .lines()
        .skip_while(|line| !line.starts_with("on:"))
        .skip(1)
        .take_while(|line| line.starts_with(' ') || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(on_block.contains("workflow_dispatch:"));
    for trigger in [
        "push:",
        "pull_request",
        "schedule:",
        "workflow_run:",
        "repository_dispatch:",
    ] {
        assert!(!on_block.contains(trigger), "unexpected trigger {trigger}");
    }
    assert!(
        text.lines()
            .any(|line| line.trim() == "environment: tea-interop"),
        "job must run in the protected tea-interop Environment"
    );
    assert!(text.contains("TEA_BASE_URL: ${{ vars.TEA_BASE_URL }}"));
}

#[test]
fn no_dispatch_input_can_choose_a_url() {
    let text = workflow();
    for forbidden in ["base_url", "inputs.tei", "well_known", "inputs.url"] {
        assert!(!text.contains(forbidden), "found `{forbidden}`");
    }
    for line in text.lines().filter(|line| line.contains("${{ inputs.")) {
        let key = line.trim().split(':').next().unwrap_or_default();
        assert!(
            matches!(key, "TEA_PRODUCT_RELEASE" | "TEA_ARTIFACT_UUID"),
            "input used outside the UUID env keys: {line}"
        );
    }
    // Secrets only ever reach the probe step's environment.
    for line in text.lines().filter(|line| line.contains("secrets.")) {
        assert!(line.trim().starts_with("TEA_API_KEY_"), "{line}");
    }
}

#[test]
fn permissions_are_least_privilege() {
    let text = workflow();
    assert!(text.lines().any(|line| line == "permissions: {}"));
    assert!(!text.contains(": write"), "no write permission is needed");
    assert!(text.contains("persist-credentials: false"));
}

// ---------------------------------------------------------------------------
// Step-level policy: where secrets go, what runs next to them, and which
// third-party actions are pinned.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Step {
    name: String,
    uses: Option<String>,
    env: Vec<(String, String)>,
    /// Shell body of `run:` with comment lines removed.
    run: String,
    /// The raw text of the whole step.
    raw: String,
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Minimal parser for the fixed shape used by `.github/workflows/*.yml`:
/// `steps:` at four spaces, each step starting with `      - ` (six spaces).
fn steps_of(text: &str) -> Vec<Step> {
    let lines: Vec<&str> = text.lines().collect();
    let mut steps: Vec<Step> = Vec::new();
    let mut in_steps = false;
    for line in &lines {
        if *line == "    steps:" {
            in_steps = true;
            continue;
        }
        if !in_steps {
            continue;
        }
        if !line.trim().is_empty() && indent(line) <= 4 {
            in_steps = false;
            continue;
        }
        // Comments belong to nobody: keep prose about secrets out of the checks.
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with("      - ") {
            steps.push(Step::default());
        }
        if let Some(step) = steps.last_mut() {
            step.raw.push_str(line);
            step.raw.push('\n');
        }
    }
    for step in &mut steps {
        let lines: Vec<&str> = step.raw.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            let body = line.trim_start().trim_start_matches("- ");
            if let Some(name) = body.strip_prefix("name: ") {
                if indent(line) <= 8 {
                    step.name = name.trim().to_owned();
                }
            } else if let Some(uses) = body.strip_prefix("uses: ") {
                step.uses = Some(uses.trim().to_owned());
            } else if body == "env:" && indent(line) <= 8 {
                let base = indent(line);
                while index + 1 < lines.len() && indent(lines[index + 1]) > base {
                    index += 1;
                    if let Some((key, value)) = lines[index].trim().split_once(": ") {
                        step.env.push((key.to_owned(), value.to_owned()));
                    }
                }
            } else if body.starts_with("run:") && indent(line) <= 8 {
                let base = indent(line);
                let inline = body.trim_start_matches("run:").trim();
                if !inline.is_empty() && inline != "|" {
                    step.run.push_str(inline);
                    step.run.push('\n');
                }
                while index + 1 < lines.len()
                    && (lines[index + 1].trim().is_empty() || indent(lines[index + 1]) > base)
                {
                    index += 1;
                    let trimmed = lines[index].trim();
                    if !trimmed.starts_with('#') {
                        step.run.push_str(trimmed);
                        step.run.push('\n');
                    }
                }
            }
            index += 1;
        }
    }
    steps
}

fn is_sha(reference: &str) -> bool {
    reference.len() == 40 && reference.bytes().all(|b| b.is_ascii_hexdigit())
}

#[test]
fn secrets_reach_exactly_one_step_and_it_runs_no_build_tool() {
    let text = workflow();
    let steps = steps_of(&text);
    assert!(steps.len() >= 5, "workflow parsed into {steps:?}");

    let with_secrets: Vec<&Step> = steps
        .iter()
        .filter(|s| s.raw.contains("secrets."))
        .collect();
    assert_eq!(
        with_secrets.len(),
        1,
        "secrets must appear on exactly one step, found {:?}",
        with_secrets.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
    let probe = with_secrets[0];
    assert_eq!(probe.name, "Run probe");
    // They are set in `env:`, never interpolated into the shell body.
    assert!(!probe.run.contains("secrets."));
    assert!(probe
        .env
        .iter()
        .filter(|(_, value)| value.contains("secrets."))
        .all(|(key, _)| key.starts_with("TEA_API_KEY_")));

    // That step runs the probe and nothing that fetches or builds code.
    let banned = [
        "cargo", "rustup", "rustc", "npm", "npx", "yarn", "pnpm", "pip", "pip3", "pipx", "uv ",
        "go ", "make", "curl", "wget", "apt", "brew", "gem ", "bundle",
    ];
    for tool in banned {
        let found = probe
            .run
            .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .any(|word| word == tool.trim());
        assert!(
            !found,
            "the secrets step must not invoke `{tool}`: {}",
            probe.run
        );
    }
    assert!(probe.run.contains("./scripts/interop-probe.sh"));
}

#[test]
fn the_cli_is_built_before_the_secrets_step_without_secrets() {
    let text = workflow();
    let steps = steps_of(&text);
    let probe_at = steps
        .iter()
        .position(|s| s.raw.contains("secrets."))
        .unwrap();
    let build_at = steps
        .iter()
        .position(|s| s.run.contains("cargo build") && s.run.contains("-p libtea-cli"))
        .expect("a step builds libtea-cli");
    assert!(
        build_at < probe_at,
        "the build must precede the secrets step"
    );
    let build = &steps[build_at];
    assert!(build.run.contains("--locked"));
    assert!(build.run.contains("--release"));
    assert!(
        !build.raw.contains("secrets."),
        "the build step must have no secrets"
    );
    // It publishes the binary path for later steps.
    assert!(
        build.run.contains("TEA_CLI_BIN=") && build.run.contains("$GITHUB_ENV"),
        "{}",
        build.run
    );
    // Nothing between the build and the probe re-exports credentials.
    for step in &steps[..probe_at] {
        assert!(!step.raw.contains("TEA_API_KEY"), "{}", step.name);
    }
}

/// Third-party actions should be pinned to a full commit SHA. The ones below
/// are not yet; the list is the explicit debt. The test fails if a new
/// unpinned action appears, or when an entry is pinned and should be removed.
#[test]
fn unpinned_third_party_actions_are_exactly_the_known_debt() {
    const KNOWN_UNPINNED: &[(&str, &str)] = &[
        ("ci.yml", "taiki-e/install-action@cargo-audit"),
        ("ci.yml", "taiki-e/install-action@cargo-semver-checks"),
        ("interop-manual.yml", "actions/upload-artifact@v4"),
        ("spec-drift.yml", "actions/upload-artifact@v4"),
    ];
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows");
    let mut unpinned: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        for step in steps_of(&std::fs::read_to_string(&path).unwrap()) {
            if let Some(uses) = step.uses {
                let (action, reference) = uses.split_once('@').expect("uses: owner/repo@ref");
                if !is_sha(reference) {
                    unpinned.push((file.clone(), format!("{action}@{reference}")));
                }
            }
        }
    }
    unpinned.sort();
    unpinned.dedup();
    for (file, uses) in &unpinned {
        eprintln!("UNPINNED action in {file}: {uses}");
    }
    let mut known: Vec<(String, String)> = KNOWN_UNPINNED
        .iter()
        .map(|(f, u)| ((*f).to_owned(), (*u).to_owned()))
        .collect();
    known.sort();
    assert_eq!(unpinned, known);
}

#[test]
fn the_step_parser_sees_the_steps_it_is_asserting_on() {
    // Guard against the policy tests passing because nothing was parsed.
    let steps = steps_of(&workflow());
    let names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();
    for expected in [
        "Checkout",
        "Build CLI (no secrets)",
        "Run probe",
        "Upload results",
    ] {
        assert!(names.contains(&expected), "{names:?}");
    }
    let probe = steps.iter().find(|s| s.name == "Run probe").unwrap();
    assert!(probe.env.iter().any(|(k, _)| k == "TEA_API_KEY_SECRET"));
    assert!(probe.env.iter().any(|(k, _)| k == "TEA_OUT_DIR"));
}
