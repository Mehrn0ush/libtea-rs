#!/usr/bin/env bash
# Creates one GitHub issue per milestone work item.
# DO NOT RUN until you have read it: it writes to GitHub (labels, milestones, issues).
# Prerequisites: `gh auth status` is logged in; run from the repository root.
set -euo pipefail

REPO="${REPO:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"

# Labels (idempotent).
gh label create gate:spec --repo "$REPO" --force >/dev/null
gh label create gate:API --repo "$REPO" --force >/dev/null
gh label create gate:interop --repo "$REPO" --force >/dev/null
gh label create gate:security --repo "$REPO" --force >/dev/null
gh label create 'gate:DevSecOps usability' --repo "$REPO" --force >/dev/null
gh label create gate:maintenance --repo "$REPO" --force >/dev/null
gh label create size:S --repo "$REPO" --force >/dev/null
gh label create size:M --repo "$REPO" --force >/dev/null
gh label create size:L --repo "$REPO" --force >/dev/null

# Milestones (ignore "already exists").
gh api "repos/$REPO/milestones" -f title=0.1 -f description='0.1.0 — first publish' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.2 -f description='0.2 — compatibility and API decisions, first interop pass, operations basics' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.3 -f description='0.3 — CLI for CI' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.4 -f description='0.4 — purl, cache, offline mode, allowlist' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.5 -f description='0.5 — CLE evaluation and adapters' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.6 -f description='0.6 — signature verification' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=0.7 -f description='0.7 — hardening and external review' >/dev/null 2>&1 || true
gh api "repos/$REPO/milestones" -f title=1.0 -f description='1.0.0 — gates and decision record' >/dev/null 2>&1 || true

# 0.1-1
gh issue create --repo "$REPO" \
  --title '0.1-1 Commit the audited tree and get CI green on the branch' \
  --milestone 0.1 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Nothing has run on GitHub yet; `HEAD` is still `599e513` and the audited work is uncommitted.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] `git status --short` is empty after the commits in `HANDOFF-COMMANDS.md`
- [ ] every commit passes `cargo check --workspace --all-targets --locked` (`git rebase -x` proof)
- [ ] CI jobs `rust stable`, `rust 1.85.0`, `package`, `cargo deny`, `cargo semver-checks` are green; `cargo semver-checks` logs "not on crates.io yet" for both crates
- [ ] the `Doctests` step passes

Source: docs/milestones/0.1.md#01-1
ISSUE_BODY

# 0.1-2
gh issue create --repo "$REPO" \
  --title '0.1-2 Create the `tea-interop` GitHub Environment' \
  --milestone 0.1 \
  --label gate:security --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** The interop workflow reads its target and secrets from this Environment; until it exists the workflow cannot run and the protection is only on paper.

**Size:** S (≤ 2 days)
**Gate:** security
**Depends on:** none

**Acceptance criteria**
- [ ] Environment exists with required reviewers (self-review prevented), `main` only, admin bypass off
- [ ] `TEA_BASE_URL` is an `https://` variable; any `TEA_API_KEY_*` are Environment secrets, not repository secrets
- [ ] `gh api repos/<owner>/<repo>/environments/tea-interop` output is pasted into the release PR

Source: docs/milestones/0.1.md#01-2
ISSUE_BODY

# 0.1-3
gh issue create --repo "$REPO" \
  --title '0.1-3 Fill the BEFORE test counts' \
  --milestone 0.1 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `REPORT.md` keeps the base-commit counts as PENDING.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] `HANDOFF-COMMANDS.md` section a is run; five `BEFORE_*` numbers replace PENDING in `REPORT.md`

Source: docs/milestones/0.1.md#01-3
ISSUE_BODY

# 0.1-4
gh issue create --repo "$REPO" \
  --title '0.1-4 Publish `libtea-core`, then `libtea-client`' \
  --milestone 0.1 \
  --label gate:API --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Creates the semver baseline.

**Size:** S (≤ 2 days)
**Gate:** API
**Depends on:** 0.1-1

**Acceptance criteria**
- [ ] order follows `docs/release-checklist.md`: dry-run core, tag the `main` SHA, publish core, wait for the index, dry-run client, publish client
- [ ] a fresh project builds against both crates; docs.rs shows both crates with the independence and checksum≠authenticity banners
- [ ] the tag points at the `main` merge commit

Source: docs/milestones/0.1.md#01-4
ISSUE_BODY

# 0.1-5
gh issue create --repo "$REPO" \
  --title '0.1-5 Make `cargo semver-checks` a required status check' \
  --milestone 0.1 \
  --label gate:API --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** It skips on a 404 today; after publish it must gate.

**Size:** S (≤ 2 days)
**Gate:** API
**Depends on:** 0.1-4

**Acceptance criteria**
- [ ] branch protection on `main` lists `cargo semver-checks` as required
- [ ] a deliberate breaking change on a scratch branch fails the job

Source: docs/milestones/0.1.md#01-5
ISSUE_BODY

# 0.1-6
gh issue create --repo "$REPO" \
  --title '0.1-6 Record the live Vulnetix run of 2026-11-08' \
  --milestone 0.1 \
  --label gate:interop --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** The maintainer ran discover + download against Vulnetix; the repository should hold the sanitized result, not only a remark.

**Size:** S (≤ 2 days)
**Gate:** interop
**Depends on:** none

**Acceptance criteria**
- [ ] `scripts/interop-probe.sh` result file (secrets absent, `auth_mode: anonymous`) is stored under `docs/interop/` (see `HANDOFF-COMMANDS.md`)
- [ ] the placeholder in `docs/interoperability.md` is replaced by the file name and its `timestamp_utc`

Source: docs/milestones/0.1.md#01-6
ISSUE_BODY

# 0.2-1
gh issue create --repo "$REPO" \
  --title '0.2-1 Write the compatibility policy' \
  --milestone 0.2 \
  --label gate:spec --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** 0.1.0 ships `lenient_versions` (default off), tolerant discovery parsing, and a `urn:tei:` error as defaults chosen by the maintainer; the reasoning and the re-pin rules are not written down.

**Size:** S (≤ 2 days)
**Gate:** spec
**Depends on:** none

**Acceptance criteria**
- [ ] `docs/compatibility.md` states: supported spec versions, what `lenient_versions` accepts and the risk, unknown-field policy per document, how a re-pin changes the version policy
- [ ] every rule names the test that pins it

Source: docs/milestones/0.2.md#02-1
ISSUE_BODY

# 0.2-2
gh issue create --repo "$REPO" \
  --title '0.2-2 Re-pin dry-run report' \
  --milestone 0.2 \
  --label gate:spec --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Re-pinning is manual; `spec-drift.yml` only diffs `openapi.yaml`.

**Size:** M (about 1–2 weeks)
**Gate:** spec
**Depends on:** none

**Acceptance criteria**
- [ ] a script compares two OpenAPI files and lists added/removed operations, schemas, required fields and enum values
- [ ] running it on the pinned file against itself reports nothing; on a synthetic edit it lists the change
- [ ] its output is linked from `docs/re-pinning.md`

Source: docs/milestones/0.2.md#02-2
ISSUE_BODY

# 0.2-3
gh issue create --repo "$REPO" \
  --title '0.2-3 API freeze review' \
  --milestone 0.2 \
  --label gate:API --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** The model structs are now `#[non_exhaustive]`, and `url`/`semver`/`uuid` are re-exported; remaining open points: the `ComplianceDocument` tuple struct, `Checksum`, the experimental labels on `follow_pages` and `SignatureVerifier`, and whether `Service` method signatures can stay as they are.

**Size:** M (about 1–2 weeks)
**Gate:** API
**Depends on:** 0.1-4

**Acceptance criteria**
- [ ] each public item is listed in `docs/api-inventory.md` with `non_exhaustive`, third-party types, and stability (stable / experimental)
- [ ] `cargo semver-checks` against 0.1.0 passes or each break is a recorded decision
- [ ] experimental items either graduate or keep the label with a removal date

Source: docs/milestones/0.2.md#02-3
ISSUE_BODY

# 0.2-4
gh issue create --repo "$REPO" \
  --title '0.2-4 Interop pass: ReARM' \
  --milestone 0.2 \
  --label gate:interop --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Listed by the spec; never executed.

**Size:** M (about 1–2 weeks)
**Gate:** interop
**Depends on:** 0.1-6

**Acceptance criteria**
- [ ] a probe configuration (env block, expected product release) is in `docs/interop/rearm.md`
- [ ] a dated, sanitized result file from `scripts/interop-probe.sh` is checked in, or the failure is documented with the response status and a minimal reproduction
- [ ] discrepancies become issues, not silent workarounds

Source: docs/milestones/0.2.md#02-4
ISSUE_BODY

# 0.2-5
gh issue create --repo "$REPO" \
  --title '0.2-5 Interop pass: sbomify' \
  --milestone 0.2 \
  --label gate:interop --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Listed by the spec; never executed.

**Size:** M (about 1–2 weeks)
**Gate:** interop
**Depends on:** 0.1-6

**Acceptance criteria**
- [ ] same acceptance as 0.2-4 for sbomify

Source: docs/milestones/0.2.md#02-5
ISSUE_BODY

# 0.2-6
gh issue create --repo "$REPO" \
  --title '0.2-6 Interop pass: Cybeats' \
  --milestone 0.2 \
  --label gate:interop --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** One `.well-known` fetch (2026-10-10) showed `versions: ["1"]`. 0.1.0 can read that only with `lenient_versions(true)`.

**Size:** S (≤ 2 days)
**Gate:** interop
**Depends on:** 0.1-6

**Acceptance criteria**
- [ ] probe run with the flag off (expected: clear version error) and on; both outcomes recorded with dates
- [ ] whether `/v1/` serves TEA 1.0.0 responses the models read is recorded, including any parse failure

Source: docs/milestones/0.2.md#02-6
ISSUE_BODY

# 0.2-7
gh issue create --repo "$REPO" \
  --title '0.2-7 Interop pass: a CLE-capable server' \
  --milestone 0.2 \
  --label gate:interop --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Oolong answers `NOT_IMPLEMENTED`; no server with real CLE data has been exercised.

**Size:** M (about 1–2 weeks)
**Gate:** interop
**Depends on:** none

**Acceptance criteria**
- [ ] at least one server returning CLE events is probed; the response parses into `Cle`; result recorded
- [ ] if none is found, the search and the reason are recorded and 0.5 carries the risk

Source: docs/milestones/0.2.md#02-7
ISSUE_BODY

# 0.2-8
gh issue create --repo "$REPO" \
  --title '0.2-8 Live-mode conformance runner' \
  --milestone 0.2 \
  --label gate:interop --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** `libtea-conformance` is offline only and cannot be pointed at a server.

**Size:** M (about 1–2 weeks)
**Gate:** interop
**Depends on:** 0.2-4

**Acceptance criteria**
- [ ] `libtea conformance --base <url> [--allow-origin]` runs read-only checks (discovery shape, pagination, error bodies) and prints a JSON report with `is_certification: false`
- [ ] strict-schema lint warnings (`lint_*_strict`) appear in `warnings`, never in failures
- [ ] runs against the wiremock stub in a test

Source: docs/milestones/0.2.md#02-8
ISSUE_BODY

# 0.2-9
gh issue create --repo "$REPO" \
  --title '0.2-9 `Retry-After` and 429 handling' \
  --milestone 0.2 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Only transport errors and 503 are retried; 429 surfaces as an API error.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** none

**Acceptance criteria**
- [ ] 429 and 503 with `Retry-After` (seconds and HTTP-date) wait at most `Limits::max_retry_after` and then retry within `max_retries`
- [ ] a `Retry-After` above the cap fails fast with a typed error
- [ ] wiremock tests for both forms, the cap, and non-idempotent methods (not retried)

Source: docs/milestones/0.2.md#02-9
ISSUE_BODY

# 0.2-10
gh issue create --repo "$REPO" \
  --title '0.2-10 CLI tracing subscriber with `-v`/`-q`' \
  --milestone 0.2 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** The library emits redacted `tracing` events; the CLI never shows them.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** none

**Acceptance criteria**
- [ ] `-v` / `-vv` / `-q` flags and `RUST_LOG` select levels; default output is unchanged
- [ ] a test runs a flow with a secret key and a token and asserts neither appears at `-vv` (stderr and stdout)

Source: docs/milestones/0.2.md#02-10
ISSUE_BODY

# 0.2-11
gh issue create --repo "$REPO" \
  --title '0.2-11 Scheduled fuzzing in CI' \
  --milestone 0.2 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Six fuzz targets exist; none run in CI.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] a scheduled workflow runs every target for a fixed time with the checked-in corpus and dictionary
- [ ] a crash uploads the artifact and fails the job
- [ ] workflow has `permissions: {}` at top and `contents: read` on the job

Source: docs/milestones/0.2.md#02-11
ISSUE_BODY

# 0.2-12
gh issue create --repo "$REPO" \
  --title '0.2-12 Dependabot' \
  --milestone 0.2 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** No dependency update automation.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] `.github/dependabot.yml` covers `cargo` (root and `fuzz/`) and `github-actions`
- [ ] MSRV pins in `Cargo.toml` (`url`, `idna`, `uuid`, `wiremock`) are documented as ignore rules

Source: docs/milestones/0.2.md#02-12
ISSUE_BODY

# 0.2-13
gh issue create --repo "$REPO" \
  --title '0.2-13 Pin remaining GitHub Actions by SHA; decide `cargo audit` gating' \
  --milestone 0.2 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `actions/upload-artifact@v4` and two `taiki-e/install-action` refs are tags; the `audit` job is non-gating.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] no `uses:` in `.github/workflows/` references a tag or branch
- [ ] `cargo audit` is either required or its non-gating status is explained next to the job

Source: docs/milestones/0.2.md#02-13
ISSUE_BODY

# 0.2-14
gh issue create --repo "$REPO" \
  --title '0.2-14 Signature design decisions (ADR)' \
  --milestone 0.2 \
  --label gate:security --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Signatures are the longest-lead item; four decisions in `docs/open-questions.md` block them.

**Size:** S (≤ 2 days)
**Gate:** security
**Depends on:** none

**Acceptance criteria**
- [ ] `docs/design/signature-adr.md` fixes: first formats, trust-anchor sourcing, digest binding, Sigstore as feature or crate
- [ ] `docs/open-questions.md` items 1–4 are marked resolved

Source: docs/milestones/0.2.md#02-14
ISSUE_BODY

# 0.3-1
gh issue create --repo "$REPO" \
  --title '0.3-1 Documented, tested JSON schemas' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Output shapes are ad hoc `serde_json::json!` values with no schema or test.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** none

**Acceptance criteria**
- [ ] `docs/cli-output.md` and `docs/schemas/*.json` describe every `--json` output and the `conformance` report
- [ ] a test per command validates real output against its schema
- [ ] a change to a field name fails a test

Source: docs/milestones/0.3.md#03-1
ISSUE_BODY

# 0.3-2
gh issue create --repo "$REPO" \
  --title '0.3-2 Documented exit codes' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** Codes today: 0, 1, 4; not in `--help` or the README; no code separates auth, not-found, integrity, or policy failures.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** 0.3-1

**Acceptance criteria**
- [ ] table in `docs/cli-output.md` and in `libtea --help`
- [ ] distinct codes for usage, network, auth, not found, policy refusal, integrity failure, weak-only checksums (4 is kept)
- [ ] a test asserts each code

Source: docs/milestones/0.3.md#03-2
ISSUE_BODY

# 0.3-3
gh issue create --repo "$REPO" \
  --title '0.3-3 `download` JSON status' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `download` reports on stderr in prose.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** 0.3-1

**Acceptance criteria**
- [ ] `--json` prints one object (artifact id, revision, media type, bytes, checksum results, strong ETag, final URL, outcome) to stderr or a file given by `--status-file`; stdout stays the artifact bytes
- [ ] covered by the schema test

Source: docs/milestones/0.3.md#03-3
ISSUE_BODY

# 0.3-4
gh issue create --repo "$REPO" \
  --title '0.3-4 VEX/SBOM selection helper' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** The caller must walk release → collection → artifacts and filter by type.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** none

**Acceptance criteria**
- [ ] library function selects artifacts by `ArtifactType` and media type from a collection, returning every match in a stable order
- [ ] CLI `libtea fetch --release <uuid> --type VULNERABILITIES|BOM [--media-type …]`
- [ ] tests: none, one, several matches; unknown type string preserved

Source: docs/milestones/0.3.md#03-4
ISSUE_BODY

# 0.3-5
gh issue create --repo "$REPO" \
  --title '0.3-5 Streaming download with incremental hashing' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** The artifact is buffered in a `Vec` (up to 256 MiB × `max_in_flight`) and hashed afterwards.

**Size:** L (more than 2 weeks)
**Gate:** DevSecOps usability
**Depends on:** none

**Acceptance criteria**
- [ ] new API writes to a caller-supplied sink (file) while feeding `ChecksumVerifier`
- [ ] peak memory stays below a stated bound in a test with a body larger than the bound
- [ ] a checksum mismatch removes the partial file; size limits apply to encoded and decoded bytes while streaming

Source: docs/milestones/0.3.md#03-5
ISSUE_BODY

# 0.3-6
gh issue create --repo "$REPO" \
  --title '0.3-6 Resume with `Range`' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** No resume.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-5

**Acceptance criteria**
- [ ] an interrupted download continues with `Range` and `If-Range`/ETag; a changed ETag restarts
- [ ] hash is computed over the whole file, including the already-written prefix
- [ ] wiremock tests for 206, 200 (server ignores Range), 416, and ETag change

Source: docs/milestones/0.3.md#03-6
ISSUE_BODY

# 0.3-7
gh issue create --repo "$REPO" \
  --title '0.3-7 Lock file' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** No record of what was fetched.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-4, 0.3-5

**Acceptance criteria**
- [ ] `tea.lock` (documented, schema-tested) records artifact id and revision, URL, media type, checksums, bytes, server base URL, API version, spec commit, client version, timestamp, ETag
- [ ] written atomically; deterministic ordering; no secrets
- [ ] round-trip test

Source: docs/milestones/0.3.md#03-7
ISSUE_BODY

# 0.3-8
gh issue create --repo "$REPO" \
  --title '0.3-8 `sync` and `diff`' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** No way to refresh a lock or see what changed.

**Size:** L (more than 2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-7, 0.3-2

**Acceptance criteria**
- [ ] `libtea diff` compares a lock to the server and prints added/changed/removed entries (JSON and text); exit code says whether anything changed
- [ ] `libtea sync` downloads what changed, verifies, rewrites the lock; `--frozen` fails if the lock would change
- [ ] conditional requests are used where ETags are present

Source: docs/milestones/0.3.md#03-8
ISSUE_BODY

# 0.3-9
gh issue create --repo "$REPO" \
  --title '0.3-9 Publish the CLI' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** `libtea-cli` is `publish = false`; the Action and binaries need a decision on name and distribution.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-8

**Acceptance criteria**
- [ ] decision recorded (`docs/open-questions.md` #5); either crates.io publish or GitHub-release-only
- [ ] release workflow builds Linux/macOS/Windows binaries with checksums
- [ ] a container image is built from the release binary

Source: docs/milestones/0.3.md#03-9
ISSUE_BODY

# 0.3-10
gh issue create --repo "$REPO" \
  --title '0.3-10 GitHub Action' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** No Action.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-9

**Acceptance criteria**
- [ ] `action.yml` downloads a release binary, verifies its checksum, runs `sync --frozen` or `fetch`
- [ ] example workflow in `docs/` runs in a test repository
- [ ] credentials only from secrets via environment

Source: docs/milestones/0.3.md#03-10
ISSUE_BODY

# 0.3-11
gh issue create --repo "$REPO" \
  --title '0.3-11 Pipeline example' \
  --milestone 0.3 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** No CI example.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** 0.3-10

**Acceptance criteria**
- [ ] one example workflow in `examples/ci/` fetches an SBOM and fails on integrity failure
- [ ] the workflow is linted in CI (actionlint or equivalent)

Source: docs/milestones/0.3.md#03-11
ISSUE_BODY

# 0.4-1
gh issue create --repo "$REPO" \
  --title '0.4-1 purl parsing and canonicalization' \
  --milestone 0.4 \
  --label gate:spec --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** There is no purl type; `validate_purl` only checks the `pkg:` prefix after Base64URL decoding.

**Size:** M (about 1–2 weeks)
**Gate:** spec
**Depends on:** none

**Acceptance criteria**
- [ ] a maintained purl crate (or a recorded decision to hand-roll) is chosen with `cargo deny` and MSRV 1.85 checked
- [ ] library function turns a purl into a canonical TEI (`purl` type, Base64URL, no padding) and back
- [ ] the official purl-spec test suite runs in CI against it; failures are listed, not hidden

Source: docs/milestones/0.4.md#04-1
ISSUE_BODY

# 0.4-2
gh issue create --repo "$REPO" \
  --title '0.4-2 Persistent ETag cache' \
  --milestone 0.4 \
  --label 'gate:DevSecOps usability' --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** ETags are returned but not stored; no cache.

**Size:** L (more than 2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-7

**Acceptance criteria**
- [ ] opt-in on-disk cache keyed by URL and credentials scope, storing ETag and body hash
- [ ] conditional requests; eviction by size and age; `Cache-Control: no-store` honored
- [ ] security review note: cache directory permissions, poisoning, credentials never stored

Source: docs/milestones/0.4.md#04-2
ISSUE_BODY

# 0.4-3
gh issue create --repo "$REPO" \
  --title '0.4-3 Offline mode' \
  --milestone 0.4 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Not available.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.4-2

**Acceptance criteria**
- [ ] `--offline` / `ClientBuilder::offline(true)` serves only from the cache and the lock; any network need is a typed error
- [ ] test with a stub that fails on any request

Source: docs/milestones/0.4.md#04-3
ISSUE_BODY

# 0.4-4
gh issue create --repo "$REPO" \
  --title '0.4-4 Host allowlist' \
  --milestone 0.4 \
  --label gate:security --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `DestinationPolicy` lists exceptions; it cannot say "only these hosts".

**Size:** S (≤ 2 days)
**Gate:** security
**Depends on:** none

**Acceptance criteria**
- [ ] `DestinationPolicy::only_hosts` (or equivalent) refuses every other host, including after redirects and from discovery
- [ ] CLI `--allow-host`
- [ ] tests for discovery pointing elsewhere and a redirect to another host

Source: docs/milestones/0.4.md#04-4
ISSUE_BODY

# 0.4-5
gh issue create --repo "$REPO" \
  --title '0.4-5 Optional HTTPS/SVCB lookup' \
  --milestone 0.4 \
  --label gate:spec --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Not implemented.

**Size:** M (about 1–2 weeks)
**Gate:** spec
**Depends on:** none

**Acceptance criteria**
- [ ] either implemented behind a feature with a resolver dependency review, or the roadmap row records "not planned" with the reason
- [ ] if implemented: records are validated against `DestinationPolicy`

Source: docs/milestones/0.4.md#04-5
ISSUE_BODY

# 0.4-6
gh issue create --repo "$REPO" \
  --title '0.4-6 Record `swid` as a non-registered TEI type' \
  --milestone 0.4 \
  --label gate:spec --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `grep -rni swid spec/vendor` finds nothing, so `swid` is not in the pinned registry; unknown types stay structurally valid.

**Size:** S (≤ 2 days)
**Gate:** spec
**Depends on:** none

**Acceptance criteria**
- [ ] `docs/specification-baseline.md` states it and a test pins that an unknown type such as `swid` parses and validates

Source: docs/milestones/0.4.md#04-6
ISSUE_BODY

# 0.5-1
gh issue create --repo "$REPO" \
  --title '0.5-1 CLE evaluation API' \
  --milestone 0.5 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** CLE is fetched and typed, but nothing answers "is component version V supported / end of life / unknown?".

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.2-7

**Acceptance criteria**
- [ ] `Cle::status_at(version, instant) -> LifecycleStatus` (`Supported`, `EndOfSupport`, `EndOfLife`, `Withdrawn`, `Unknown`, `#[non_exhaustive]`)
- [ ] handles `versions` specifiers, `supersededBy`, and unknown event types without panicking
- [ ] table-driven tests from the ECMA-428 examples; property test over arbitrary event orders

Source: docs/milestones/0.5.md#05-1
ISSUE_BODY

# 0.5-2
gh issue create --repo "$REPO" \
  --title '0.5-2 `libtea cle` command' \
  --milestone 0.5 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** No CLI access to CLE.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** 0.5-1, 0.3-1

**Acceptance criteria**
- [ ] `libtea cle --component … --version … [--at …] --json` prints status; exit code reflects status with `--fail-on eol`
- [ ] schema-tested

Source: docs/milestones/0.5.md#05-2
ISSUE_BODY

# 0.5-3
gh issue create --repo "$REPO" \
  --title '0.5-3 Scanner adapter examples' \
  --milestone 0.5 \
  --label 'gate:DevSecOps usability' --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** No example connects fetched SBOM/VEX to a scanner.

**Size:** M (about 1–2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-8

**Acceptance criteria**
- [ ] runnable examples for grype and osv-scanner that take a lock file entry and pass the verified file
- [ ] a Dependency-Track upload example
- [ ] each example states that it only uses integrity-checked bytes, not authenticated ones

Source: docs/milestones/0.5.md#05-3
ISSUE_BODY

# 0.5-4
gh issue create --repo "$REPO" \
  --title '0.5-4 End-to-end tutorial' \
  --milestone 0.5 \
  --label 'gate:DevSecOps usability' --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** README has snippets only.

**Size:** S (≤ 2 days)
**Gate:** DevSecOps usability
**Depends on:** 0.5-3

**Acceptance criteria**
- [ ] `docs/tutorial.md`: discover → select → download → verify → lock → scan → CLE check; every snippet is compiled or run in CI

Source: docs/milestones/0.5.md#05-4
ISSUE_BODY

# 0.6-1
gh issue create --repo "$REPO" \
  --title '0.6-1 Verifier API redesign' \
  --milestone 0.6 \
  --label gate:security --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** The experimental trait has no trust policy, identity, digest binding, or link to downloads.

**Size:** M (about 1–2 weeks)
**Gate:** security
**Depends on:** 0.2-14

**Acceptance criteria**
- [ ] new API carries trust policy and expected identity, distinguishes valid-and-trusted from valid-but-untrusted, and attaches the result to the download
- [ ] old experimental items are removed or deprecated with a note in `CHANGELOG.md`
- [ ] `SignatureVerifyOutcome::Verified` can only be produced under a caller-supplied policy

Source: docs/milestones/0.6.md#06-1
ISSUE_BODY

# 0.6-2
gh issue create --repo "$REPO" \
  --title '0.6-2 First verifier (caller-supplied keys)' \
  --milestone 0.6 \
  --label gate:security --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** No verifier exists.

**Size:** L (more than 2 weeks)
**Gate:** security
**Depends on:** 0.6-1

**Acceptance criteria**
- [ ] feature-gated verifier for the format fixed by the ADR, with caller-supplied trust anchors only
- [ ] test vectors: good/trusted, good/untrusted, modified artifact, modified signature, wrong signer, unsupported algorithm, missing anchors, expired certificate (if PKIX)
- [ ] no trust anchor is ever fetched from the host that served the artifact

Source: docs/milestones/0.6.md#06-2
ISSUE_BODY

# 0.6-3
gh issue create --repo "$REPO" \
  --title '0.6-3 Lock file and CLI integration' \
  --milestone 0.6 \
  --label gate:security --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** The lock cannot record verification.

**Size:** M (about 1–2 weeks)
**Gate:** security
**Depends on:** 0.6-2, 0.3-7

**Acceptance criteria**
- [ ] lock entries record signer identity and policy id; `sync --frozen` fails when verification changes
- [ ] CLI `--trust-root`; exit codes for invalid vs untrusted

Source: docs/milestones/0.6.md#06-3
ISSUE_BODY

# 0.6-4
gh issue create --repo "$REPO" \
  --title '0.6-4 Dependency, MSRV, and license review of the verifier stack' \
  --milestone 0.6 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** New crypto dependencies must keep MSRV 1.85 and `cargo deny` green.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** 0.6-2

**Acceptance criteria**
- [ ] `cargo +1.85.0 test` and `cargo deny check` pass with the feature on
- [ ] `docs/dependency-weight.md` updated

Source: docs/milestones/0.6.md#06-4
ISSUE_BODY

# 0.6-5
gh issue create --repo "$REPO" \
  --title '0.6-5 Optional Sigstore path' \
  --milestone 0.6 \
  --label gate:security --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** Out of the first verifier.

**Size:** L (more than 2 weeks)
**Gate:** security
**Depends on:** 0.6-2

**Acceptance criteria**
- [ ] either a separate feature with its own policy inputs, or a recorded decision to leave it out of 1.0

Source: docs/milestones/0.6.md#06-5
ISSUE_BODY

# 0.7-1
gh issue create --repo "$REPO" \
  --title '0.7-1 External security review' \
  --milestone 0.7 \
  --label gate:security --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** One reviewer (the author and agents) so far.

**Size:** L (more than 2 weeks)
**Gate:** security
**Depends on:** 0.6-2

**Acceptance criteria**
- [ ] written review of `transport.rs`, `policy.rs`, checksum/decoder code, CLI file handling, and the verifier by a person outside the project
- [ ] findings tracked as issues; none Critical/High open

Source: docs/milestones/0.7.md#07-1
ISSUE_BODY

# 0.7-2
gh issue create --repo "$REPO" \
  --title '0.7-2 Signed, reproducible releases with provenance' \
  --milestone 0.7 \
  --label gate:maintenance --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** No release workflow, signing, or provenance.

**Size:** M (about 1–2 weeks)
**Gate:** maintenance
**Depends on:** 0.3-9

**Acceptance criteria**
- [ ] release workflow with build provenance attestation and published checksums
- [ ] two independent builds of the CLI binary produce identical hashes, or the differences are documented

Source: docs/milestones/0.7.md#07-2
ISSUE_BODY

# 0.7-3
gh issue create --repo "$REPO" \
  --title '0.7-3 SBOM of libtea-rs' \
  --milestone 0.7 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `vulnetix-sbom.cdx.json` is a local, ignored file.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** 0.7-2

**Acceptance criteria**
- [ ] a CycloneDX SBOM is generated in CI and attached to each release

Source: docs/milestones/0.7.md#07-3
ISSUE_BODY

# 0.7-4
gh issue create --repo "$REPO" \
  --title '0.7-4 Fuzz depth' \
  --milestone 0.7 \
  --label gate:security --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Targets run for minutes.

**Size:** M (about 1–2 weeks)
**Gate:** security
**Depends on:** 0.2-11, 0.5-1, 0.4-1

**Acceptance criteria**
- [ ] corpus checked in and minimized; coverage report; the CLE evaluator, lock parser, and purl code have targets
- [ ] a 24-hour run of each target finishes without a crash before 1.0

Source: docs/milestones/0.7.md#07-4
ISSUE_BODY

# 0.7-5
gh issue create --repo "$REPO" \
  --title '0.7-5 Security process' \
  --milestone 0.7 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** `SECURITY.md` names a channel only.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** none

**Acceptance criteria**
- [ ] response and disclosure timeline stated; a private advisory template exists; supported-version table is current

Source: docs/milestones/0.7.md#07-5
ISSUE_BODY

# 0.7-6
gh issue create --repo "$REPO" \
  --title '0.7-6 MSRV and re-pin policies' \
  --milestone 0.7 \
  --label gate:maintenance --label size:S \
  --body-file - <<'ISSUE_BODY'
**Why:** MSRV pins exist but no written policy.

**Size:** S (≤ 2 days)
**Gate:** maintenance
**Depends on:** 0.2-1

**Acceptance criteria**
- [ ] `docs/maintenance.md` states when MSRV changes and how a spec re-pin maps to crate versions

Source: docs/milestones/0.7.md#07-6
ISSUE_BODY

# 1.0-1
gh issue create --repo "$REPO" \
  --title '1.0-1 Gate: spec' \
  --milestone 1.0 \
  --label gate:spec --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Compatibility rules are written, tested and survived at least one real re-pin.

**Size:** M (about 1–2 weeks)
**Gate:** spec
**Depends on:** 0.2-1, 0.2-2

**Acceptance criteria**
- [ ] `docs/compatibility.md` unchanged for a full release cycle
- [ ] one re-pin (or a rehearsal on a synthetic spec change) done using the dry-run report

Source: docs/milestones/1.0.md#10-1
ISSUE_BODY

# 1.0-2
gh issue create --repo "$REPO" \
  --title '1.0-2 Gate: API' \
  --milestone 1.0 \
  --label gate:API --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Public API frozen.

**Size:** M (about 1–2 weeks)
**Gate:** API
**Depends on:** 0.2-3

**Acceptance criteria**
- [ ] `cargo semver-checks` is required and passed for two consecutive releases
- [ ] no item is labelled experimental

Source: docs/milestones/1.0.md#10-2
ISSUE_BODY

# 1.0-3
gh issue create --repo "$REPO" \
  --title '1.0-3 Gate: interop' \
  --milestone 1.0 \
  --label gate:interop --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** Evidence beyond two servers.

**Size:** L (more than 2 weeks)
**Gate:** interop
**Depends on:** 0.2-4, 0.2-5, 0.2-6, 0.2-7

**Acceptance criteria**
- [ ] dated, reproducible probe results for at least four independent servers (including a CLE-capable one), kept in `docs/interop/`

Source: docs/milestones/1.0.md#10-3
ISSUE_BODY

# 1.0-4
gh issue create --repo "$REPO" \
  --title '1.0-4 Gate: security' \
  --milestone 1.0 \
  --label gate:security --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** Authenticity story, external review, release integrity.

**Size:** L (more than 2 weeks)
**Gate:** security
**Depends on:** 0.6-3, 0.7-1, 0.7-2

**Acceptance criteria**
- [ ] 0.6 and 0.7 items closed
- [ ] no open Critical/High findings

Source: docs/milestones/1.0.md#10-4
ISSUE_BODY

# 1.0-5
gh issue create --repo "$REPO" \
  --title '1.0-5 Gate: DevSecOps usability' \
  --milestone 1.0 \
  --label 'gate:DevSecOps usability' --label size:L \
  --body-file - <<'ISSUE_BODY'
**Why:** CLI, lock, Action, binaries, tutorial.

**Size:** L (more than 2 weeks)
**Gate:** DevSecOps usability
**Depends on:** 0.3-10, 0.5-4

**Acceptance criteria**
- [ ] 0.3 and 0.5 items closed; the tutorial runs in CI

Source: docs/milestones/1.0.md#10-5
ISSUE_BODY

# 1.0-6
gh issue create --repo "$REPO" \
  --title '1.0-6 Gate: maintenance' \
  --milestone 1.0 \
  --label gate:maintenance --label size:M \
  --body-file - <<'ISSUE_BODY'
**Why:** Process in place.

**Size:** M (about 1–2 weeks)
**Gate:** maintenance
**Depends on:** 0.2-11, 0.2-12, 0.7-5

**Acceptance criteria**
- [ ] Dependabot active, scheduled fuzzing green for 30 days, security process published

Source: docs/milestones/1.0.md#10-6
ISSUE_BODY
