//! Policy lint for `.github/workflows/interop-manual.yml`.
//!
//! The workflow hands repository secrets to the probe, so the target URL must
//! come from the protected Environment, not from whoever dispatches it.

use std::path::{Path, PathBuf};

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

/// Third-party actions should be pinned to a full commit SHA. The list below
/// is the explicit debt (empty since every action is pinned). The test fails if
/// a new unpinned action appears, or when an entry is pinned and should be
/// removed.
#[test]
fn unpinned_third_party_actions_are_exactly_the_known_debt() {
    const KNOWN_UNPINNED: &[(&str, &str)] = &[];
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
                // A pinned SHA is followed by a `# vX.Y.Z` comment.
                let reference = reference.split(" # ").next().unwrap();
                // PIN_ME_* placeholders are policed by the pin gate
                // (pin_gate.rs, scripts/check-pins.sh), not listed here.
                if !is_sha(reference) && !reference.starts_with("PIN_ME_") {
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

/// The release binaries are built on named runner images, so the glibc baseline
/// of the gnu archives is a decision and not whatever `-latest` points to.
#[test]
fn the_release_workflow_builds_on_named_runner_images() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/release.yml"),
    )
    .unwrap();
    let images: Vec<&str> = text
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("runs-on: ")
                .or_else(|| l.strip_prefix("os: "))
        })
        .collect();
    assert!(images.len() >= 10, "{images:?}");
    for image in &images {
        assert!(
            image.contains("matrix.os") || !image.contains("latest"),
            "release.yml uses a floating runner image: {image}"
        );
    }
    assert!(text.contains("os: ubuntu-24.04\n"), "gnu baseline image");
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

// ---------------------------------------------------------------------------
// .github/workflows/release.yml: tag-triggered release binaries.
//
// The release workflow signs nothing with a secret and publishes nothing to
// crates.io. These tests pin down who may write, which jobs may hold a token,
// and that third-party actions are pinned (or are an explicit placeholder).
// ---------------------------------------------------------------------------

use std::collections::BTreeMap;

fn release() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/release.yml");
    std::fs::read_to_string(path).unwrap()
}

struct Job {
    id: String,
    raw: String,
}

/// Split the `jobs:` section into jobs (ids at two spaces of indent).
fn jobs_of(text: &str) -> Vec<Job> {
    let mut jobs: Vec<Job> = Vec::new();
    let mut in_jobs = false;
    for line in text.lines() {
        if line == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if !line.trim().is_empty() && indent(line) == 0 {
            break;
        }
        if indent(line) == 2 && line.trim_end().ends_with(':') && !line.trim().starts_with('#') {
            jobs.push(Job {
                id: line.trim().trim_end_matches(':').to_owned(),
                raw: String::new(),
            });
        }
        if let Some(job) = jobs.last_mut() {
            job.raw.push_str(line);
            job.raw.push('\n');
        }
    }
    jobs
}

/// `permissions:` of a job (indent 4) as key -> value.
fn job_permissions(job: &Job) -> BTreeMap<String, String> {
    let lines: Vec<&str> = job.raw.lines().collect();
    let mut map = BTreeMap::new();
    let Some(start) = lines.iter().position(|l| *l == "    permissions:") else {
        panic!("job {} has no permissions block", job.id);
    };
    for line in &lines[start + 1..] {
        if indent(line) <= 4 {
            break;
        }
        let (key, value) = line.trim().split_once(": ").unwrap();
        map.insert(key.to_owned(), value.to_owned());
    }
    map
}

fn job<'a>(jobs: &'a [Job], id: &str) -> &'a Job {
    jobs.iter()
        .find(|j| j.id == id)
        .unwrap_or_else(|| panic!("no job {id}"))
}

fn on_block(text: &str) -> String {
    text.lines()
        .skip_while(|line| !line.starts_with("on:"))
        .skip(1)
        .take_while(|line| line.starts_with(' ') || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn release_jobs_are_the_expected_ones() {
    let jobs = jobs_of(&release());
    let ids: Vec<&str> = jobs.iter().map(|j| j.id.as_str()).collect();
    assert_eq!(ids, ["meta", "build", "sbom", "attest", "draft-release"]);
    for job in &jobs {
        assert!(
            job.raw.contains("    timeout-minutes: "),
            "{} has no timeout",
            job.id
        );
    }
}

#[test]
fn release_runs_on_tags_only() {
    let text = release();
    let on = on_block(&text);
    assert!(on.contains("push:") && on.contains("tags:"), "{on}");
    for forbidden in [
        "branches",
        "pull_request",
        "pull_request_target",
        "schedule",
        "workflow_dispatch",
        "workflow_run",
        "repository_dispatch",
        "release:",
        "issue_comment",
    ] {
        assert!(
            !on.contains(forbidden),
            "release.yml must not trigger on {forbidden}"
        );
    }
    assert!(!text.contains("pull_request_target"));
    assert!(text.contains("cancel-in-progress: false"));
}

#[test]
fn release_permissions_are_least_privilege() {
    let text = release();
    assert!(
        text.lines().any(|line| line == "permissions: {}"),
        "top-level permissions must be empty"
    );
    let jobs = jobs_of(&text);
    let read_only = BTreeMap::from([("contents".to_owned(), "read".to_owned())]);
    for id in ["meta", "build", "sbom"] {
        assert_eq!(job_permissions(job(&jobs, id)), read_only, "{id}");
    }
    assert_eq!(
        job_permissions(job(&jobs, "attest")),
        BTreeMap::from([
            ("attestations".to_owned(), "write".to_owned()),
            ("contents".to_owned(), "read".to_owned()),
            ("id-token".to_owned(), "write".to_owned()),
        ])
    );
    assert_eq!(
        job_permissions(job(&jobs, "draft-release")),
        BTreeMap::from([("contents".to_owned(), "write".to_owned())])
    );
    // No write scope outside those two jobs.
    for j in &jobs {
        if !matches!(j.id.as_str(), "attest" | "draft-release") {
            assert!(
                !j.raw.contains(": write"),
                "{} must not have write scopes",
                j.id
            );
        }
    }
    // The release environment gates both jobs that hold more than read access:
    // the one that signs (attest) and the one that creates the draft.
    for id in ["attest", "draft-release"] {
        assert!(
            job(&jobs, id).raw.contains("    environment: release\n"),
            "{id} must run in the release Environment"
        );
    }
    for id in ["meta", "build", "sbom"] {
        assert!(!job(&jobs, id).raw.contains("environment:"), "{id}");
    }
}

#[test]
fn release_holds_no_secret_and_scopes_the_token() {
    let text = release();
    let jobs = jobs_of(&text);
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        assert!(
            !step.raw.contains("secrets."),
            "step `{}` uses a secret",
            step.name
        );
    }
    assert!(!text
        .lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains("secrets.")));
    for j in &jobs {
        let uses_token = j.raw.lines().any(|l| {
            !l.trim_start().starts_with('#')
                && (l.contains("github.token")
                    || l.contains("GITHUB_TOKEN")
                    || l.contains("GH_TOKEN"))
        });
        assert_eq!(uses_token, j.id == "draft-release", "token use in {}", j.id);
        let uses_oidc = j.raw.contains("id-token");
        assert_eq!(uses_oidc, j.id == "attest", "id-token in {}", j.id);
    }
    // Build jobs have nothing that could carry credentials into the build.
    let build = job(&jobs, "build");
    for forbidden in ["GH_TOKEN", "TOKEN", "SECRET", "KEY"] {
        let steps = steps_of(&build.raw);
        for step in &steps {
            for (key, _) in &step.env {
                assert!(
                    !key.contains(forbidden),
                    "build env `{key}` looks like a credential"
                );
            }
        }
    }
}

#[test]
fn release_never_publishes_to_a_registry() {
    let jobs = jobs_of(&release());
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        for banned in [
            "cargo publish",
            "cargo login",
            "CARGO_REGISTRY_TOKEN",
            "twine",
            "npm publish",
            "docker push",
        ] {
            assert!(
                !step.run.contains(banned) && !step.raw.contains(banned),
                "`{banned}` in step `{}`",
                step.name
            );
        }
    }
}

#[test]
fn release_run_bodies_interpolate_no_expressions() {
    // Everything attacker-influenced (the tag name) reaches a script through
    // `env:`, never through `${{ }}` inside the shell text.
    let jobs = jobs_of(&release());
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        assert!(
            !step.run.contains("${{"),
            "expression in run body of `{}`: {}",
            step.name,
            step.run
        );
    }
}

#[test]
fn release_builds_exactly_the_documented_targets() {
    let jobs = jobs_of(&release());
    let build = job(&jobs, "build");
    let mut targets: Vec<String> = build
        .raw
        .lines()
        .filter_map(|l| {
            l.trim()
                .trim_start_matches("- ")
                .strip_prefix("target: ")
                .map(str::to_owned)
        })
        .collect();
    targets.sort();
    assert_eq!(
        targets,
        [
            "aarch64-apple-darwin",
            "aarch64-unknown-linux-gnu",
            "aarch64-unknown-linux-musl",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
        ]
    );
    let steps = steps_of(&build.raw);
    let compile = steps
        .iter()
        .find(|s| s.run.contains("cargo build"))
        .expect("a build step");
    assert!(compile.run.contains("--locked") && compile.run.contains("--release"));
    assert!(compile.run.contains("-p libtea-cli"));
    assert!(steps
        .iter()
        .any(|s| s.run.contains("scripts/package-release.py")));
}

#[test]
fn release_checkouts_do_not_persist_credentials() {
    let jobs = jobs_of(&release());
    let mut checkouts = 0;
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        if step
            .uses
            .as_deref()
            .is_some_and(|u| u.starts_with("actions/checkout@"))
        {
            checkouts += 1;
            assert!(
                step.raw.contains("persist-credentials: false"),
                "{}",
                step.name
            );
        }
    }
    assert!(checkouts >= 3);
    // The jobs that hold elevated tokens never check out code.
    for id in ["attest", "draft-release"] {
        assert!(
            !job(&jobs, id).raw.contains("actions/checkout"),
            "{id} must not check out source"
        );
    }
}

#[test]
fn release_privileged_jobs_run_no_build_tools() {
    let jobs = jobs_of(&release());
    for id in ["attest", "draft-release"] {
        for step in steps_of(&job(&jobs, id).raw) {
            for tool in [
                "cargo",
                "rustup",
                "rustc",
                "make",
                "npm",
                "pip",
                "python",
                "python3",
                "curl",
                "wget",
                "apt-get",
                "bash scripts",
                "./scripts",
            ] {
                let found = step
                    .run
                    .split(|c: char| {
                        !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
                    })
                    .any(|word| word == tool);
                assert!(!found, "`{tool}` in {id} step `{}`", step.name);
            }
        }
    }
}

#[test]
fn release_actions_are_pinned_or_explicit_placeholders() {
    const ALLOWED: &[&str] = &[
        "actions/checkout",
        "dtolnay/rust-toolchain",
        "actions/upload-artifact",
        "actions/download-artifact",
        "actions/attest-build-provenance",
    ];
    let jobs = jobs_of(&release());
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        let Some(uses) = step.uses else { continue };
        let (action, reference) = uses.split_once('@').expect("owner/repo@ref");
        assert!(ALLOWED.contains(&action), "unexpected action {action}");
        let reference = reference.split_whitespace().next().unwrap_or_default();
        assert!(
            is_sha(reference) || reference.starts_with("PIN_ME_"),
            "{action} is pinned to `{reference}`, not a commit SHA"
        );
    }
}

#[test]
fn release_produces_checksums_sbom_provenance_and_only_a_draft() {
    let jobs = jobs_of(&release());
    let sbom = steps_of(&job(&jobs, "sbom").raw);
    assert!(sbom.iter().any(|s| s.run.contains("scripts/make-sbom.sh")));
    assert!(sbom.iter().any(|s| s
        .run
        .contains("cargo install cargo-cyclonedx --locked --version")));

    let attest = steps_of(&job(&jobs, "attest").raw);
    assert!(attest
        .iter()
        .any(|s| s.run.contains("sha256sum") && s.run.contains("SHA256SUMS")));
    let provenance = attest
        .iter()
        .find(|s| {
            s.uses
                .as_deref()
                .is_some_and(|u| u.starts_with("actions/attest-build-provenance@"))
        })
        .expect("an attestation step");
    assert!(provenance.raw.contains("subject-path: dist/*"));

    let draft = steps_of(&job(&jobs, "draft-release").raw);
    let create = draft
        .iter()
        .find(|s| s.run.contains("gh release create"))
        .expect("release step");
    assert!(create.run.contains("--draft") && create.run.contains("--verify-tag"));
    assert!(!create.run.contains("--latest"));
    assert!(draft
        .iter()
        .any(|s| s.run.contains("sha256sum --check --strict")));
    // Only the draft job creates a release.
    for j in &jobs {
        if j.id != "draft-release" {
            assert!(
                !steps_of(&j.raw)
                    .iter()
                    .any(|s| s.run.contains("gh release")),
                "{}",
                j.id
            );
        }
    }
}

#[test]
fn release_tag_flows_through_env_and_the_validator() {
    let jobs = jobs_of(&release());
    let meta = steps_of(&job(&jobs, "meta").raw);
    let tag = meta
        .iter()
        .find(|s| s.run.contains("check-release-tag.sh"))
        .unwrap();
    assert!(tag
        .env
        .iter()
        .any(|(k, v)| k == "TAG" && v == "${{ github.ref_name }}"));
    assert!(tag.run.contains("\"$TAG\""));
}

#[test]
fn every_release_tag_must_be_on_main_dry_runs_included() {
    let jobs = jobs_of(&release());
    let meta = steps_of(&job(&jobs, "meta").raw);
    let on_main = meta
        .iter()
        .find(|s| s.run.contains("merge-base --is-ancestor"))
        .expect("a step checks that the tagged commit is on main");
    assert!(on_main.run.contains("origin/main") && on_main.run.contains("$GITHUB_SHA"));
    // No condition: the check runs for dry-run tags too.
    assert!(
        !on_main.run.contains("DRY_RUN") && !on_main.run.contains("if "),
        "{}",
        on_main.run
    );
    assert!(!on_main.raw.contains("DRY_RUN"), "{}", on_main.raw);
}

#[test]
fn release_never_publishes_the_github_release() {
    let jobs = jobs_of(&release());
    let mut creates = 0;
    for step in jobs.iter().flat_map(|j| steps_of(&j.raw)) {
        for forbidden in [
            "gh release edit",
            "gh release upload",
            "--draft=false",
            "--latest",
            "gh api",
            "releases/",
            "prerelease=false",
        ] {
            assert!(
                !step.run.contains(forbidden),
                "`{forbidden}` in step `{}`",
                step.name
            );
        }
        if step.run.contains("gh release create") {
            creates += 1;
            assert!(step.run.contains("--draft"), "{}", step.run);
        }
    }
    assert_eq!(creates, 1, "exactly one step creates the release");
}

// Runs a bash snippet; CI runs these on Linux and macOS (bash on Windows is WSL or Git Bash, which these scripts never run under).
#[cfg(unix)]
fn run_name_check(version: &str, files: &[String]) -> std::process::Output {
    let jobs = jobs_of(&release());
    let attest = steps_of(&job(&jobs, "attest").raw);
    let step = attest
        .iter()
        .find(|s| s.name == "Check the file names")
        .expect("the attest job checks the file names");
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "libtea-names-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("dist")).unwrap();
    for file in files {
        std::fs::write(dir.join("dist").join(file), b"x").unwrap();
    }
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(&step.run)
        .current_dir(&dir)
        .env("VERSION", version)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    output
}

// Runs a bash snippet; CI runs these on Linux and macOS (bash on Windows is WSL or Git Bash, which these scripts never run under).
#[cfg(unix)]
fn expected_release_files(version: &str) -> Vec<String> {
    let mut files: Vec<String> = [
        "aarch64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
    ]
    .iter()
    .map(|t| format!("libtea-{version}-{t}.tar.gz"))
    .collect();
    files.push(format!("libtea-{version}-x86_64-pc-windows-msvc.zip"));
    files.push(format!("libtea-{version}.cdx.json"));
    files
}

// Runs a bash snippet; CI runs these on Linux and macOS (bash on Windows is WSL or Git Bash, which these scripts never run under).
#[cfg(unix)]
#[test]
fn the_attest_job_accepts_exactly_the_expected_file_names() {
    for version in ["0.1.0", "0.1.0-dryrun.1"] {
        let ok = run_name_check(version, &expected_release_files(version));
        assert!(
            ok.status.success(),
            "{version}: {}",
            String::from_utf8_lossy(&ok.stderr)
        );
    }
}

// Runs a bash snippet; CI runs these on Linux and macOS (bash on Windows is WSL or Git Bash, which these scripts never run under).
#[cfg(unix)]
#[test]
fn the_attest_job_rejects_any_other_set_of_files() {
    let good = expected_release_files("0.1.0");
    // A rejection must be the check's own message, not some unrelated failure.
    let rejected = |out: std::process::Output, what: &str| {
        assert!(!out.status.success(), "{what} passed");
        assert!(
            String::from_utf8_lossy(&out.stderr)
                .contains("does not hold exactly the expected files"),
            "{what}: rejected for another reason: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let with = |edit: &dyn Fn(&mut Vec<String>)| {
        let mut files = good.clone();
        edit(&mut files);
        run_name_check("0.1.0", &files)
    };
    // The old check only counted files: eight files with the wrong names passed.
    rejected(
        with(&|f| f[0] = "evil.tar.gz".to_owned()),
        "a misnamed file with the right count",
    );
    rejected(with(&|f| f.truncate(7)), "a missing file");
    rejected(with(&|f| f.push("extra.bin".to_owned())), "an extra file");
    rejected(
        with(&|f| f[1] = "libtea-0.0.9-aarch64-unknown-linux-gnu.tar.gz".to_owned()),
        "an older version's archive",
    );
    rejected(
        with(&|f| f[2] = "libtea-0.1.0-dryrun.1-aarch64-unknown-linux-musl.tar.gz".to_owned()),
        "a dry-run archive as the release",
    );
    rejected(
        with(&|f| f.push("SHA256SUMS".to_owned())),
        "a stray SHA256SUMS",
    );
    // A different version than the tag.
    rejected(run_name_check("0.2.0", &good), "files for another version");
}

#[test]
fn the_name_check_runs_before_anything_is_checksummed_or_attested() {
    let jobs = jobs_of(&release());
    let attest = steps_of(&job(&jobs, "attest").raw);
    let at = |name: &str| {
        attest
            .iter()
            .position(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert!(at("Check the file names") < at("SHA-256 checksums"));
    assert!(at("SHA-256 checksums") < at("Attest build provenance"));
    assert!(attest[at("Check the file names")]
        .env
        .iter()
        .any(|(k, v)| k == "VERSION" && v.contains("needs.meta.outputs.version")));
}

/// The manual interop workflow stops early, naming the accepted inputs, when
/// nothing is selected to probe; and that step never sees the API key.
#[test]
fn the_interop_workflow_checks_that_something_is_selected_before_the_probe() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/interop-manual.yml"),
    )
    .unwrap();
    let check = text
        .find("name: Check that something is selected to probe")
        .expect("the check step exists");
    let probe = text.find("name: Run probe").expect("the probe step exists");
    assert!(check < probe, "the check runs before the probe");
    let step = &text[check..probe];
    assert!(
        !step.contains("TEA_API_KEY"),
        "the check never sees the key"
    );
    for needed in [
        "::error::",
        "TEA_TEI",
        "TEA_PRODUCT_RELEASE",
        "TEA_ARTIFACT_UUID",
        "product_release",
        "artifact_uuid",
        "exit 1",
    ] {
        assert!(step.contains(needed), "check step lacks `{needed}`");
    }
    for line in step.lines().filter(|l| l.trim().starts_with("echo")) {
        assert!(
            !line.contains("${{"),
            "no expression inside shell text: {line}"
        );
    }
}

/// Every SHA pin says what it is, and `docs/release-pins.md` records it.
///
/// The comment is `# vX.Y.Z` for an action with release tags. For
/// `dtolnay/rust-toolchain`, whose only tag `v1` moves with `master`, it is
/// `# v1 YYYY-MM-DD` or `# master YYYY-MM-DD` (the commit date). Every SHA must
/// also be a row of the pin table in `docs/release-pins.md`, with the release
/// in the comment's first word, so a bump (by hand or by Dependabot) that
/// changes the SHA or leaves a stale comment fails here until the table is
/// updated.
#[test]
fn every_sha_pin_names_its_release() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pins_doc = std::fs::read_to_string(root.join("docs/release-pins.md")).unwrap();
    // `| <what> | <where> | `<sha>` | <release> |` rows of the pin table.
    let table: Vec<(String, String)> = pins_doc
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 6 {
                return None;
            }
            let sha = cells[3].trim_matches('`');
            is_sha(sha).then(|| (sha.to_owned(), cells[4].to_owned()))
        })
        .collect();
    assert!(
        table.len() >= 8,
        "pin table not found in docs/release-pins.md"
    );

    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    files.push(root.join("action.yml"));
    let is_date = |text: &str| {
        let b = text.as_bytes();
        b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    };
    let is_version = |text: &str| {
        text.strip_prefix('v').is_some_and(|rest| {
            rest.split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    let mut checked = 0;
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            let Some(uses) = line
                .trim()
                .strip_prefix("uses: ")
                .or_else(|| line.trim().strip_prefix("- uses: "))
            else {
                continue;
            };
            let Some((action, rest)) = uses.split_once('@') else {
                continue;
            };
            let (reference, comment) = rest.split_once(" # ").unwrap_or((rest, ""));
            if !is_sha(reference) {
                continue;
            }
            checked += 1;
            let words: Vec<&str> = comment.split_whitespace().collect();
            let ok = if action == "dtolnay/rust-toolchain" {
                words.len() == 2
                    && (words[0] == "master" || is_version(words[0]))
                    && is_date(words[1])
            } else {
                words.len() == 1 && is_version(words[0])
            };
            assert!(
                ok,
                "{}: {uses} needs a `# vX.Y.Z` comment (rust-toolchain: `# v1 YYYY-MM-DD` or `# master YYYY-MM-DD`)",
                path.display()
            );
            let release = words[0];
            assert!(
                table
                    .iter()
                    .any(|(sha, recorded)| sha == reference && recorded == release),
                "{}: {action}@{reference} ({release}) is not a row of the pin table in docs/release-pins.md",
                path.display()
            );
        }
    }
    assert!(
        checked > 10,
        "only {checked} SHA pins found; the parser is wrong"
    );
}
