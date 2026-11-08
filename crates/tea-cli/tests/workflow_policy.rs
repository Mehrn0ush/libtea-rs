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
        // release.yml: unresolved pins. They fail at job setup until
        // docs/release-pins.md is followed; remove the entries when pinned.
        (
            "release.yml",
            "actions/attest-build-provenance@PIN_ME_ATTEST_BUILD_PROVENANCE",
        ),
        (
            "release.yml",
            "actions/download-artifact@PIN_ME_DOWNLOAD_ARTIFACT",
        ),
        (
            "release.yml",
            "actions/upload-artifact@PIN_ME_UPLOAD_ARTIFACT",
        ),
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
    // The release environment gates the job that can create a release.
    assert!(job(&jobs, "draft-release")
        .raw
        .contains("    environment: release\n"));
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

/// Run before tagging: fails while any pin or the cargo-cyclonedx version is
/// still a placeholder. Ignored by default so the tree stays green until
/// `docs/release-pins.md` is followed; `docs/release-checklist.md` runs it.
#[test]
#[ignore = "fails until docs/release-pins.md is followed; run with --ignored before tagging"]
fn release_workflow_has_no_unresolved_pins() {
    let text = release();
    let left: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("PIN_ME_") && !l.trim_start().starts_with('#'))
        .collect();
    assert!(left.is_empty(), "unresolved pins:\n{}", left.join("\n"));
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
