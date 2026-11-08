# Overclaim sweep (verification pass)

Case-insensitive grep for: verified, verify, authentic, trusted, trust,
compatible, compatibility, certified, certification, conformant, secure,
guarantee, always, never — across rustdoc, README, docs, CHANGELOG, CLI,
conformance strings, and comments (excluding `fuzz/crates/` mirror noise).

## Changed hits

| File:approx | Before (summary) | After | Reason |
| --- | --- | --- | --- |
| `docs/auth.md` | Bound stated without empty-cache / no mixed-retry conditions | Bound scoped to empty/expired cache, same origin, first exchange succeeds; mutex client-wide; not lifetime exactly-once | Match N=32 test + transport |
| `crates/tea-client/src/lib.rs` crate docs | Short single-flight sentence | Same conditions as `docs/auth.md` | Consistency |
| `crates/tea-client/src/transport.rs` `token_refresh` docs | Broad “guarantee” of 1 exchange | Bound + conditions; “guarantee” wording softened | Match test |
| `crates/tea-client/src/service.rs` `DownloadedArtifact::integrity` | “every declared checksum matched” | “…matched **decoded** artifact bytes — not publisher authenticity” | Qualify Verified |
| `crates/tea-client/src/service.rs` `download_format` | “checksums are verified over…” | “checked… (integrity of decoded bytes, not authenticity)” | Avoid “verified” alone |
| `crates/tea-cli/src/main.rs` download stderr | `integrity verified\t…` | Adds `(decoded-byte checksums only; not publisher authenticity)` | Qualify CLI claim |
| `crates/tea-cli/tests/download.rs` | assert contains `integrity verified` | Also asserts authenticity disclaimer present | Keep test honest |
| `README.md` Vulnetix download note | “prints `integrity verified`” | Mentions decoded-byte / not authenticity | Match CLI |
| `scripts/interop-probe.sh` | Stored `verified` / raw CLI out | `checksum_match_decoded_bytes`; sanitize secrets; chmod 600 | Safety + honesty |
| `CHANGELOG.md` | Broad token bound | Empty-cache conditions; experimental markers | Accuracy |
| `crates/tea-client/src/page_stream.rs` / `signature.rs` | No experimental note | **Experimental: may change in a 0.x minor** | T7 |

## Unchanged hits (justification)

| Hit | Justification |
| --- | --- |
| `IntegrityOutcome::Verified` enum / checksum code | Domain term; rustdoc/README already qualify ≠ authenticity |
| `SignatureVerifyOutcome::Verified` / “no verifier returns Verified” | Negative claim; tested |
| `certificate verification` / TLS trust anchors | Literal TLS; “cannot be disabled” is accurate |
| Matrix / interop “not certification / not compatibility” | Honesty posture |
| `conformance` crate / CLI command name | Smoke suite; report sets `is_certification: false` |
| Auth tests / “authentication required” | Protocol wording |
| DestinationPolicy / “untrusted TEA responses” Limits docs | Size bounds on untrusted input |
| `always restore` env in CLI tests | Test hygiene, not product claim |
| Design docs discussing trust anchors / authenticity | Threat-model language; no shipped crypto claim |
| `docs/security.md` content-coding “Unknown… are errors” | Behaviour under test |

## Residual risk

Grep volume is large (~500+ hits). Future edits that print `Verified` without the decoded-bytes qualifier should be rejected in review.

## 2026-11-07 audit sweep

Same terms, re-run over `README.md`, crate READMEs, `CHANGELOG.md`,
`SECURITY.md`, `docs/`, rustdoc in `crates/*/src`, CLI strings, and scripts
(excluding `spec/vendor/` and test code).

### Changed

| File | Before | After | Reason |
| --- | --- | --- | --- |
| `crates/tea-core/src/checksum.rs` `IntegrityOutcome::Verified` rustdoc | "Every declared checksum was computed and matched." | Adds "over the decoded artifact bytes … content integrity only, not publisher authenticity" | Unqualified `Verified` on the most-read enum |
| `crates/tea-client/src/error.rs` `ClientError::Integrity` rustdoc | "Declared checksums were not fully verified." | "did not all match the decoded artifact bytes, or at least one could not be computed" | Says what was compared |
| `CHANGELOG.md` Security | "never implied authenticity" | "matches the decoded artifact bytes; it does not imply publisher authenticity" | Drops the absolute; adds the decoded-bytes scope |
| `crates/tea-cli/src/main.rs` `conformance` help | "offline conformance checks" | "offline smoke checks … (not a certification)" | Matches `is_certification: false` |
| `README.md` heading | "Verify (CLI and examples)" | "Try it (CLI and examples)" | The section runs commands; it does not verify anything |
| `docs/auth.md`, `crates/tea-client/src/lib.rs`, `transport.rs`, `README.md`, `docs/security.md`, `docs/interoperability.md` | "client-wide mutex"; bound conditions that contradicted the exception list | Per-origin locks; bound stated with its two conditions; exceptions are a just-stored token already expired, a later call after a failure, cancellation, rejected bearer, other origin | Behavior changed (AUD-03); old text was self-contradictory |
| `scripts/interop-probe.sh` | Pass decided by grepping stderr for `integrity verified` | Pass decided by exit status of `download --require-checksums` | Prose coupling removed |
| `docs/release-checklist.md` | "CI green … plus … audit" | `cargo audit` is `continue-on-error`; `cargo deny` gates advisories | The old text claimed a gate that does not exist |
| `src/page_stream.rs` `PageFollowLimits::max_items` | "Maximum number of items to yield" | Stop after the page on which the count reaches it; may exceed by one page | The old text overclaimed the bound |

### Kept, with reason

| Hit | Reason |
| --- | --- |
| `docs/conformance-gap-analysis.md` "Verified" status column | Test-status label defined at the top of that file, not an integrity or certification claim |
| `docs/audit-report.md` line 51 "never verifies artifact bytes" | Historical finding from 2026-10-10, fixed by TEA-AUD-005 in the same file |
| `SECURITY.md` / README "certificate verification cannot be disabled" | True: no `danger_accept_invalid_certs` call exists; `add_root_pem` only adds anchors |
| "always `false`" for `is_certification` | Literal; asserted by `report_is_not_a_certification_and_names_the_pinned_commit` |
| `SignatureVerifyOutcome::Verified` | Exists for caller-supplied verifiers; no built-in code path returns it (grep: only the enum definition and a negative test) |

## Streaming download, CLI JSON and exit codes (F1)

| File | Claim | Why it is honest |
| --- | --- | --- |
| `crates/tea-client/src/download.rs` module docs | "Nothing is released as verified before the check completes" | Stated for the **file** methods only (temporary file, rename after the checksum policy passed; removed on failure and cancellation; tests in `tests/streaming_download.rs`). The **writer** method is documented as handing out unverified bytes. |
| same | "Verified means checksum integrity" | Repeats that it is not publisher authenticity; no signature check exists. |
| same | "Resume is not supported" | Negative claim; there is no `Range` code. Reasons recorded in `docs/open-questions.md`. |
| `docs/cli-json.md` `integrity.scope` | "decoded artifact bytes; not publisher authenticity" | Fixed string in every `download --json` object. |
| `docs/cli-exit-codes.md` code 0 / 6 | "exit 0 means checksum integrity of decoded bytes" | Unchanged from the existing `--require-checksums` wording; `NotDeclared` is reported on stderr. |
| `docs/cli-exit-codes.md` code 7 | "Reserved" | No command returns it; a test checks only that `--help` lists it. |
| `libtea --help` | Exit status list | Asserted by `help_documents_every_exit_status`. |
| memory bound | Not claimed | The file path does not buffer the body, but no test measures peak memory, so no bound is stated. |

## Lock file, sync and diff (F2)

| File | Claim | Why it is honest |
| --- | --- | --- |
| `docs/lock-file.md` "What it is, and what it is not" | A lock is unsigned and not authenticity | States it; `integrity: Verified` is defined as declared digests matching decoded bytes. |
| `docs/lock-file.md` | "It holds no secrets" | `LockFile::validate` rejects a `server` with userinfo, query or fragment (unit test); `no_credential_reaches_the_lock_or_the_output` runs with credentials set. The content URL is not recorded. |
| `docs/lock-file.md` | "Deterministic" | Unit test (input order), proptest (order and duplicates), and a CLI test (a second lock from the same server is byte-identical). |
| `docs/lock-file.md` | "Atomic", "symlink refused" | Temporary file plus rename; `a_symlinked_lock_is_never_written_through`. A power-loss guarantee is not claimed (the lock is `sync_all`ed before the rename, nothing more is tested). |
| `docs/lock-file.md` | "Fails closed" | `tampered_bytes_fail_closed_and_leave_the_lock_alone` (no lock created; an existing one unchanged; exit 6 under `--check`). |
| `docs/lock-file.md` `integrity` values | `NotDeclared` means nothing was checked | Tested; open question 23. |
| `docs/cli-exit-codes.md` code 7 | Emitted by `sync --check` and `diff` | Tested in `sync.rs`. |
| `docs/schemas/lock-v1.schema.json` | Matches the writer | `lock_schema.rs` checks key names, requiredness, closed objects, the version constant and the integrity names. It is **not** run through a JSON Schema validator (none in the workspace), so type and pattern keywords are unchecked. |

## Release workflow and installing docs (F3)

| File | Claim | Why it is honest |
| --- | --- | --- |
| `release.yml` header | "never publishes to crates.io", "holds no repository secret" | `release_never_publishes_to_a_registry` and `release_holds_no_secret_and_scopes_the_token` check the workflow text. They show the text, not the behaviour of a run (none has happened). |
| `release.yml` header | Per-job authority table | `release_permissions_are_least_privilege` asserts the exact permission map of every job. |
| `release.yml` header | "Not run yet" | True; stated in the file, `docs/release-pins.md`, `docs/installing.md` and the checklist. |
| `docs/installing.md` | `SHA256SUMS` is "not an independent proof of origin" | It travels with the archives; the page says so. |
| `docs/installing.md` | What the attestation proves | Worded as "GitHub Actions ran this workflow at a recorded commit and produced a file with that digest"; explicitly not source review, not toolchain trust. The `gh attestation verify` flags were written from memory of the CLI and are untested. |
| `docs/installing.md` | "not code-signed or notarized", "not claimed reproducible" | Negative claims. Archives are deterministic given the binary (`archives_are_deterministic_and_carry_no_host_identity`); the compiled binary is not claimed to be. |
| `docs/installing.md` | SBOM "is an inventory, not a vulnerability assessment" | Scope statement. The SBOM tool invocation is unverified (`docs/release-pins.md` step 3). |

## GitHub Action (F4)

| File | Claim | Why it is honest |
| --- | --- | --- |
| `action.yml` header | "poisoned cache entry is rejected" | `a_cached_archive_is_reused_only_when_it_matches_the_fresh_sums` (stubbed network). |
| `action.yml` header | Credentials reach only the final step | **Corrected (A2).** The earlier wording ("never inputs; only the final step sees the variables") was wrong: a calling step's `env:` is inherited by every step of a composite action, third-party ones included. Credentials are now the `api-key-id` / `api-key-secret` inputs, referenced once each, in the final step's `env:` (`credentials_are_inputs_that_only_the_final_step_reads`), and the first step refuses a non-empty `TEA_API_KEY*` in the ambient environment (`credentials_in_the_ambient_environment_stop_the_action`). That GitHub inherits a calling step's env into composite steps is taken from its documented behavior, not run here. |
| `action.yml` header | "Input values reach the scripts through env, never the shell text" | `inputs_reach_scripts_through_env_only` (no `${{` in a run line; the run lines are fixed). |
| `docs/github-action.md` status | "not run on GitHub", "tested against stubs" | True. Stubs replace curl, gh and libtea; the real flags of `gh attestation verify` and the Windows path handling are untested. |
| `docs/github-action.md` "What the action checks" | SHA-256 and attestation | Tested with stubs. The wording of what a pass proves repeats `docs/installing.md` (integrity of the download, build by this repository's workflow; not source safety). |
| `docs/github-action.md` examples | "safe to copy" | `example_workflows_are_safe_to_copy` lints the examples (no `pull_request_target`, read-only permissions, secrets only as the two env vars in a step, SHA-style pins, `persist-credentials: false`). It lints text; it does not run them. |

## Cache, offline mode and Retry-After (F6)

| File | Claim | Why it is honest |
| --- | --- | --- |
| `docs/cache.md` "Threat model" | The cache is not a trust boundary | Stated first; the format's hash is described as protection against corruption, not authentication. |
| `docs/cache.md` | Forged artifact entries fail the checksum check | `a_forged_artifact_entry_is_caught_by_the_checksum_check` (online `304`, offline buffered, offline streaming). The limit (forged metadata plus artifact) is stated next to it. |
| `docs/cache.md` | Directory `0700`, files `0600`; symlink / open directory refused | `the_cache_is_created_private`, `unsafe_cache_directories_are_refused`. Unix only; Windows is said to have no check. |
| `docs/cache.md` | "Nothing from a request is stored" | `vary_star_and_unlisted_headers_are_not_kept`, `responses_to_credentialed_requests_are_not_stored_unless_enabled` (no secret, token or key id in an entry file). Response headers are an allow-list. |
| `docs/cache.md` | Offline opens no connection | The mock server's request count is asserted unchanged (`cache.rs`, `cache_cli.rs`). |
| `docs/cache.md` | Damaged entries are treated as absent | Truncated, garbage, flipped, wrong-key, empty, oversized, symlink and directory cases; plus the core entry codec's truncation and flip tests, proptests and the `http_cache_entry` fuzz target. |
| `docs/cache.md` | "Atomic, not durable" | Temporary file plus rename; the concurrent-writer test; no `fsync` is said outright. |
| `docs/cache.md` `Retry-After` | Waits up to the cap, fails fast above it | `throttle.rs`. The wait test measures elapsed time with a 900 ms floor on a 1 s wait; it is timing-based. |
| key derivation | Different key ids and origins never share an entry | Unit test on `cache_key`; `different_keys_never_share_entries`. |

## Review fixes before 0.1.0

### A1: attestation, Environment, tags on main

| File | Claim | Why it is honest |
| --- | --- | --- |
| `docs/installing.md`, `scripts/action/verify-attestation.sh` | A verified attestation is for this tag | The check passes `--source-ref refs/tags/v$VERSION` with `--repo` and `--signer-workflow`; `the_attestation_must_be_for_this_release_tag` reads the stub `gh` call. Whether `gh` honors every flag as documented is **not** verified (no network, no real attestation). |
| same | An archive of another version is refused | `a_version_that_does_not_match_the_archive_is_refused_before_gh_runs`, `a_dry_run_archive_is_not_accepted_as_the_release_it_prefixes`. The check is on the file name and the tag ref, not on the archive's contents. |
| `release.yml` | Every tag, dry runs included, is on main | `every_release_tag_must_be_on_main_dry_runs_included` checks the step text. Running it needs GitHub. |
| `release.yml` | Exactly the expected files are attested | `the_attest_job_rejects_any_other_set_of_files` executes the step's shell against good and bad `dist/` directories. |
| `release.yml` | Both privileged jobs run in the `release` Environment | The test checks the text. That the Environment has required reviewers is a repository setting (`docs/release-checklist.md`) and is **not** checked by any test. |
| `release.yml` | The workflow only ever creates a draft | `release_never_publishes_the_github_release`. Publishing is a manual step. |

### A2: credentials in the Action

| File | Claim | Why it is honest |
| --- | --- | --- |
| `docs/github-action.md` Credentials | Only the final step has the credentials | Policy test on `action.yml` (each input used once, in the run step; no other step mentions `TEA_API_KEY`, apart from the guard that only reads names). It lints text; it does not run the action on GitHub. |
| same | The action refuses ambient credentials | Script test with the stub environment, including that no value is printed. A calling workflow that sets them anyway fails at the first step. |
| same | Examples pass credentials via `with:` | `example_workflows_are_safe_to_copy` rejects `TEA_API_KEY` anywhere in an example. |

### A3: cache hits

| File | Claim | Why it is honest |
| --- | --- | --- |
| `docs/cache.md` | A hit obeys the destination policy | `a_cache_hit_obeys_this_clients_destination_policy` (buffered and streaming, with a control that serves the same entry when the origin is allowed). The check is `DestinationPolicy::check_url`, the one applied before a request; it does not repeat the DNS-level checks, which need a connection. |
| same | A hit obeys the current size limit | `a_cache_hit_obeys_the_current_size_limit_for_json`, `an_oversize_entry_is_ignored_online_and_the_server_is_asked_afresh`, `a_streamed_cache_hit_obeys_the_size_limit_too`. |
| same | A failed check evicts; age renews only after the check | `bytes_from_the_cache_that_fail_their_checksums_are_evicted`, `a_streamed_304_with_forged_bytes_evicts_instead_of_renewing`, `a_304_renews_the_entry_only_after_the_checksums_passed`, `a_json_304_renews_the_entry_at_once`. |
