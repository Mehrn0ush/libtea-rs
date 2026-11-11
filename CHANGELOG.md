# Changelog

All notable changes to this workspace are documented here. The publishable crates are `libtea-core` and `libtea-client`; `libtea-cli` and `libtea-conformance` are not published to crates.io (`publish = false`). Versions refer to the publishable crates unless noted.

Items marked **Experimental** may change in a minor release before 0.9; everything else follows semantic versioning from this release on.

## Unreleased

- `libtea-core`: **fixed** a panic in `http_cache::parse_retry_after` for a 29-byte, date-shaped value with a multi-byte character where an ASCII separator belongs (for example `"Sun, 06\u{e9}No 1994 08:49:37 GMT"`): the date parser sliced the string at fixed byte offsets before checking that they were ASCII. Non-ASCII input is now rejected (`None`) before any slicing. `libtea-client` was not affected, because it passes `Retry-After` only after `HeaderValue::to_str`, which accepts visible ASCII only; direct callers of the public function were.
- `libtea-core`: the enums generated for wire values (`IdentifierType`, `ArtifactType`, `ChecksumAlgorithm`, ...) and their variants are documented, and both publishable crates now `warn(missing_docs)`, which CI turns into an error.
- `libtea-conformance` (unpublished): integration tests for the report: case identifiers are unique and listed in the requirements matrix of `docs/interoperability.md`, the JSON shape is stable, and the strict lints ignore shapes they do not lint.
- CI (not part of any crate): `cargo audit` gates (it ran with `continue-on-error`) and is expected among the required checks of `main` by `scripts/check-repo-settings.sh`. Every third-party action is now pinned by SHA: `taiki-e/install-action` moved from the `@cargo-audit` / `@cargo-semver-checks` tags to v2.87.32 with `with: tool:`; the known-debt list of unpinned actions is empty. A new policy test requires a `# vX.Y.Z` comment on every SHA pin (`cargo-deny-action` gained `# v2.1.1`; `dtolnay/rust-toolchain`, which has no release tags, names its `master` commit date). Weekly Dependabot updates for Cargo (workspace and `fuzz/`) and GitHub Actions.
- Documentation: new `docs/compatibility.md` (TEA and Rust versions, platforms, and which of them the release smoke test, CI and the GitHub Action cover), linked from the README. Statements that were true before the first release and are not now were corrected: the release workflow and the rulesets "had not been run" (they have, for every tag from `v0.1.0` to `v0.1.5`), and several "in 0.1.0" limits now say 0.1.x. `docs/cli-json.md` states that the CLI's JSON contract is unchanged since 0.1.0 (the CLI sources are identical between `v0.1.0` and `v0.1.5`).

## 0.1.5 — 2026-11-11

- `libtea-client`: **added** `libtea_client::core`, a re-export of the whole `libtea-core` crate (`pub use libtea_core as core;`). A project that only uses the client no longer needs its own `libtea-core` dependency; the types are the same ones `libtea-core` exports. Additive: nothing was removed or renamed (`cargo semver-checks` reports no update required). The READMEs' dependency blocks and examples use it. A breaking release of `libtea-core` is now also a breaking release of `libtea-client`, which the two crates' common version already implied.
- `libtea-core`: no change besides the version number. Against `v0.1.4` the only source change in the workspace is `crates/tea-client/src/lib.rs` (`git diff --stat v0.1.4 v0.1.5 -- crates/tea-core/src crates/tea-client/src crates/tea-cli/src crates/tea-conformance/src`).
- Corrected the changelog entries of 0.1.1 to 0.1.4: the command they gave for checking that the library sources did not change used a path pattern that git does not expand, so it printed nothing whatever had changed. The claims themselves were re-checked with explicit directories and hold.
- CI (not part of any crate): the tests also run on `windows-2022` and `macos-15`. A `.gitattributes` keeps LF line endings in every checkout, because tests compare lines of `action.yml` and the workflows. Tests that run bash snippets or need Unix file permissions are Unix-only and are skipped on Windows. `scripts/check-repo-settings.sh` expects the two OS jobs among the required checks of `main`.

## 0.1.4 — 2026-11-10

No library code changed: `crates/*/src` is identical to the `v0.1.3` tag (checked with `git diff --stat v0.1.3 v0.1.4 -- crates/tea-core/src crates/tea-client/src crates/tea-cli/src crates/tea-conformance/src`).

- GitHub Action (`action.yml`, not part of any crate): the action is now named `libtea-rs`. The Marketplace refused `libtea`, because a GitHub user of that name exists. The `uses: Mehrn0ush/libtea-rs@...` line in workflows is unchanged; only the display name changed. The policy test checks the new name.

## 0.1.3 — 2026-11-10

No library code changed: `crates/*/src` is identical to the `v0.1.2` tag (checked with `git diff --stat v0.1.2 v0.1.3 -- crates/tea-core/src crates/tea-client/src crates/tea-cli/src crates/tea-conformance/src`). The crate versions are bumped so a release tag contains the metadata below.

- GitHub Action (`action.yml`, not part of any crate): added the `branding` block (icon `shield`, colour `blue`) that a GitHub Marketplace listing requires. Nothing else in the Action changed; a policy test now fails if the block, the name `libtea` or a description of at most 125 characters is missing.

## 0.1.2 — 2026-11-10

No library code changed: `crates/*/src` is identical to the `v0.1.1` tag (checked with `git diff --stat v0.1.1 v0.1.2 -- crates/tea-core/src crates/tea-client/src crates/tea-cli/src crates/tea-conformance/src`). The crate versions are bumped so the Action can be pinned to a release tag that contains the fix below.

- GitHub Action (`action.yml` and `scripts/action/`, not part of any crate): the first run from a workflow on GitHub-hosted runners found two problems. **On Windows the SHA-256 check always failed**, because Git Bash's `sha256sum` puts a backslash in front of its output when the path contains one; the hash is now read from standard input, with a test that fails without the change. **In a minimal container such as Alpine** the Action needs `bash`, `curl`, `tar` and `python3` installed first; `docs/github-action.md` says so. After the fix the Action ran successfully on `ubuntu-24.04` (gnu and musl), `ubuntu-24.04-arm`, `macos-15`, `windows-2022` and an `alpine:3.20` container. **Use 0.1.2 or later of the Action;** the `v0.1.0` and `v0.1.1` tags fail the checksum check on Windows.
- `fuzz/Cargo.lock` follows the workspace version; the fuzz corpora were minimized (`cargo fuzz cmin`).

## 0.1.1 — 2026-11-09

Documentation-only release. **No library code changed** between 0.1.0 and 0.1.1: `crates/*/src` is identical to the `v0.1.0` tag (checked with `git diff --stat v0.1.0 v0.1.1 -- crates/tea-core/src crates/tea-client/src crates/tea-cli/src crates/tea-conformance/src`); the versions in the `Cargo.toml` files and the packaged README files differ.

- Corrected the dependency blocks in the READMEs of `libtea-core` and `libtea-client` and in the root README: the examples use `libtea-core` and `tokio`, which the blocks did not list. The README shown on crates.io is the one packaged with a release, so this fix reaches crates.io with 0.1.1.
- README (install section for crates.io, release binaries and `cargo install --git --tag`; badges), `docs/installing.md` (checksum and attestation commands, glibc baseline of the gnu archives and when to use musl, macOS quarantine note), `docs/release-checklist.md` (token handling, redo path, post-publish checks), `docs/interoperability.md` (dispatch inputs of the manual interop workflow).
- `CONTRIBUTING.md`, a pull-request template and a bug-report issue template.
- Workflows (not part of any crate): `actions/checkout` and `actions/upload-artifact` moved to Node 24 releases, pinned by SHA; the release workflow names its runner images; the manual interop run stops early, listing the accepted inputs, when nothing is selected to probe; `scripts/check-repo-settings.sh` reports Actions set to `local_only`; four unused license allowances removed from `deny.toml`.

## 0.1.0 — 2026-11-09

First public release. There is no earlier version to migrate from, so nothing below is a change to an existing behavior; it is what 0.1.0 contains.

Independent implementation of a consumer for the Transparency Exchange API (TEA) 1.0.0, pinned to upstream commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` (vendored under `spec/vendor/`, see `spec/PINNED.md`). Not an official CycloneDX project release and not a conformance certificate.

### libtea-core

- Domain model for the pinned schemas: products, components, releases, collections, artifacts, formats, CLE projection, pages, identifiers, token and error bodies. The wire structs and every wire enum are `#[non_exhaustive]`; unknown enum values are preserved (`Other`). Discovery documents are parsed tolerantly (unknown fields accepted, required fields required).
- TEI parsing (`Tei`, `TeiError`, including `TeiError::LegacyUrn` for `urn:tei:` input), discovery and version selection (`ApiVersion`, `SupportedApi`, `select_endpoints`, `WellKnownDocument`, `DiscoveryInfo`), `DestinationPolicy` and `classify_ip`/`IpClass`, `origin_key`, `parse_api_base`.
- Lenient version handling, off by default: `ApiVersion::parse_lenient`, `SupportedApi::{with_lenient, is_lenient}`, `WellKnownDocument::parse_lenient`, `DiscoveryInfo::parse_list_lenient`, plus `VersionedEndpoint::new` and `Page::new`.
- Checksums over decoded artifact bytes: `Checksum`, `ChecksumVerifier` (incremental), `verify_checksums`, `IntegrityOutcome` (`Verified`, `Failed`, `Unsupported`, `Incomplete`, `NotDeclared`, `Weak`), `IntegrityReport`, `ChecksumCheck`, `ChecksumPolicy` (`allow_weak`, `new`, `with_allow_weak`), `verify_checksums_with_policy`, `ChecksumVerifier::with_policy`, `ChecksumAlgorithm::is_weak`. A match on SHA-1 alone is `Weak`, not `Verified`. HTTP content coding (`gzip`, `deflate`, `identity`) is removed with a size cap on input and output: `decode_content_coding` and the incremental `checksum::ContentDecoder` with **Experimental** `take_output`, which hands out decoded bytes as they are produced (the cap still covers the whole body).
- `escape_control_chars`: C0, DEL, C1 and bidi override characters become `\uXXXX`. Server-supplied text in error values is escaped with it.
- `ListParams` with `with_page_size`, `with_page_token`, `with_sort_field`, `with_sort_order`, `with_id_type`, `with_id_value`; re-exports of `url`, `semver`, `uuid`.
- **Experimental** `lock`: the lock file model and diff. `LockFile` (`schemaVersion` 1), `LockedArtifact`, `LockedChecksum`, `LockedIntegrity`, `LockDiff`, `FieldChange`, `ChangedArtifact`, `diff_locks`, `LOCK_SCHEMA_VERSION`, and the limits `MAX_TEI_LEN`, `MAX_VERSION_LEN`, `MAX_TYPE_LEN`, `MAX_MEDIA_TYPE_LEN`, `MAX_ALGORITHM_LEN`. Parsing is strict (unknown fields and other versions are errors), serialization is canonical and deterministic, validation refuses a `server` URL with userinfo, query or fragment, duplicate entries, and text fields that are empty, overlong, multi-line or contain control characters.
- **Experimental** `http_cache`: the pure parts of the client cache. `cache_key`, `is_cache_key`, `CacheControl` (`parse`), `NotStored` and `storage_decision`, `parse_retry_after` (seconds or IMF-fixdate), `is_storable_header` and `STORED_HEADERS`, the entry format (`CacheEntryMeta`, `EntryBuilder`, `encode_entry`, `decode_entry`, `EntryError`, `CACHE_FORMAT_VERSION`, `MAX_META_BYTES`). Entries carry their length and SHA-256; the format is strict.
- Fuzz targets: `tei_parse`, `discovery_json`, `version_negotiation`, `artifact_metadata`, `origin_policy`, `content_decoding` (one-shot, chunked and draining), `lock_parse`, `http_cache_entry`, `http_cache_headers`.

### libtea-client

- `Client`, `ClientBuilder`, `Service`: the TEA consumer operations (products, components, releases, collections, artifacts, CLE, downloads, signature downloads, `HEAD`), discovery (`Client::discover`, `ResolvedRelease`), API-key to bearer exchange with token refresh that is **single-flight per API origin** (a same-origin burst of resource `401`s with no usable cached bearer costs one `POST /token`; different origins do not wait for each other; see `docs/auth.md`), paging helpers.
- Transport: rustls only, certificate verification cannot be disabled; HTTPS and public destinations by default with origin-scoped exceptions; redirect handling with credential scoping; URL redaction in errors; environment proxies honored by default (`ClientBuilder::ignore_env_proxies(true)` to connect directly; DNS-based destination checks do not apply through a proxy).
- `ClientBuilder`: `limits`, `destination_policy`, `supported_versions`, `api_key`, `allow_discovered_credentials`, `well_known_fetch_url`, `lenient_versions` (default `false`), `allow_weak_checksums`, `ignore_env_proxies`, `add_root_pem`, and the **Experimental** `cache_dir`, `cache`, `offline`.
- `Limits` (`#[non_exhaustive]`, `with_*` setters for every field): `max_discovery_bytes`, `max_json_bytes`, `max_artifact_bytes`, `max_redirects`, `connect_timeout`, `max_retries`, `max_discovery_attempts`, `initial_backoff`, `max_in_flight`, and the three timeouts: `request_timeout` (30 s; the whole of a non-artifact request), `download_timeout` (30 min; the whole of an artifact or signature `GET`), `idle_timeout` (30 s; any single wait for more bytes). A transfer that keeps arriving is not cut off by the short request timeout; a stalled one is. `idle_timeout` and `download_timeout` must not be zero. `max_retry_after` (30 s) with `with_max_retry_after`.
- `GET`s answered `429` or `503` are retried up to `max_retries`, honoring `Retry-After` (seconds or HTTP-date) up to `max_retry_after`; a longer wait, or retries running out, ends in `ClientError::Throttled { status, retry_after_secs }`.
- `ClientError` is `#[non_exhaustive]`. Besides the transport, policy, API, integrity and token variants it has `Io(String)` (a local file operation of a download or the cache), `Offline` (offline mode and the response is not cached) and `Throttled`.
- Downloads: the buffered `download_format`, `download_artifact`, `download_latest_artifact` (`DownloadedArtifact`, `ArtifactFetch`) and **Experimental** streaming methods `download_format_to_file`, `download_artifact_to_file`, `download_latest_artifact_to_file` and `download_format_to_writer` with `StreamedDownload`, `FileDownload`, `FileDownloadOptions` (`new`, `with_overwrite`). The file methods hash while writing a private (0600) temporary file next to the destination and rename it into place only after the stream ended cleanly and the checksum policy passed; failure and cancellation remove it; a symbolic-link destination is refused; without `overwrite` the file is put in place with a hard link. The writer method hands out unverified bytes until it returns `Ok`. Range resume is not supported.
- **Experimental** persistent cache and offline mode, off by default: conditional `GET`s with `ETag`/`Last-Modified`, keyed by method, URL and a hash of origin and API key id (never the secret); `Cache-Control: no-store` and, by default, `private` are honored; responses to credentialed requests are not stored unless enabled; the directory must be private, owned by the caller and not a symbolic link; atomic writes; total-size, entry-size and age limits; damaged entries are misses; artifact checksums are re-verified on every use and a failure evicts the entry; a cache hit obeys the current destination policy and size limits; streamed downloads are cached only after they verify. `CacheConfig` (`new`, `with_max_total_bytes`, `with_max_entry_bytes`, `with_max_age`, `with_authenticated`), `CacheStats`, `cache_stat`, `cache_clear`. See `docs/cache.md`.
- **Experimental** `page_stream` (`follow_pages`, `PageFollowLimits`, `PageFollowStop`, `FollowedPage`) and the `signature` extension point (`SignatureVerifier`, `SignatureVerifyOutcome`); no verifier in this crate returns `Verified`.
- Re-export `libtea_client::url`.

### libtea-cli (`libtea`; not published to crates.io)

- `libtea --version` (`-V`) prints `libtea <version>`.
- Commands: `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, `collections`, `download`, `sync`, `diff`, `cache stat|clear`. `--json` is accepted by every command; with `--json` a failure is one JSON object on stderr. Shapes: `docs/cli-json.md`.
- Exit statuses: 0 success, 1 failure, 2 usage, 3 authentication, 4 only weak (SHA-1) checksums matched, 5 not found, 6 integrity not established, 7 changes detected (`sync --check`, `diff`). See `docs/cli-exit-codes.md`.
- `download`: `--output` streams through the client (create-new, `--force` to replace, a symbolic link at the final path component is refused, no file name is taken from server data, files are private); without `--output` the verified bytes go to stdout after the check. `--require-checksums`, `--allow-weak-checksums`, `--media-type`, `--revision`, `--json` (needs `--output`).
- `sync <TEI> --lock FILE`: resolves a TEI, downloads and verifies every artifact format, writes the lock atomically, leaves an unchanged lock alone, fails closed; `--check` exits 7 when the server differs; `--artifacts-dir` keeps the verified bytes. `diff OLD NEW` compares two locks offline. Format: `docs/lock-file.md` and `docs/schemas/lock-v1.schema.json`.
- `--cache-dir` (or `LIBTEA_CACHE_DIR`), `--offline`, `--cache-authenticated` on `discover`, `product-release`, `collections`, `download`, `sync`; `libtea cache stat|clear`.
- Credentials: `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET` (preferred) or `--api-key-id`/`--api-key-secret`. Every line the CLI prints passes through the control-character escaper.
- Distributed as release binaries built by `.github/workflows/release.yml` and, for those who build it themselves, `cargo install --git`. See `docs/installing.md`.

### libtea-conformance (not published)

- Offline smoke checks and a JSON report that embeds the pinned commit; `Report.is_certification` is always `false`. Warning-level strict-schema lints (`LintWarning`, `lint_well_known_strict`, `lint_discovery_strict`, `Report.warnings`) report fields the pinned schemas do not define and never fail a report.

### GitHub Action and release process

- `action.yml`: installs a released `libtea` binary for the runner, checks its SHA-256 against a `SHA256SUMS` fetched on every run (a cached archive must match too), verifies the build-provenance attestation for the tag `refs/tags/v<version>` when `gh` is available, caches the archive, and runs `sync` (with `check`), `diff` or another command. Credentials are the `api-key-id` and `api-key-secret` inputs, read in exactly one step; the action refuses to run if `TEA_API_KEY*` is in the ambient environment. `docs/github-action.md`.
- `.github/workflows/release.yml`: tag-triggered; refuses unresolved pins; requires the tagged commit to be on `main`; builds the CLI for seven targets; writes `SHA256SUMS` and a CycloneDX SBOM; attests build provenance; creates a **draft** release. It never publishes to crates.io and never publishes the GitHub release. Both privileged jobs run in the `release` Environment. `docs/release-checklist.md` lists the repository settings that must exist first (Environment, tag ruleset, branch protection).
- `.github/workflows/repo-settings.yml` and `scripts/check-repo-settings.sh`: report when the `main` and `v*` tag rulesets are missing or weaker than `docs/release-checklist.md` describes, and when Actions is set to `local_only` (every workflow then fails with `startup_failure`; reading that setting needs admin rights, so the scheduled workflow reports it as not verified). Read-only; the rulesets themselves are what protect the branch.
- Scripts: `scripts/package-release.py`, `scripts/check-repo-settings.sh`, `scripts/check-release-tag.sh`, `scripts/check-pins.sh`, `scripts/make-sbom.sh`, `scripts/action/*`, `scripts/interop-probe.sh`, `scripts/check-non-exhaustive-guards.sh`.
- `.github/workflows/interop-manual.yml` (dispatch-only; takes every target from the protected `tea-interop` Environment; builds the CLI in a step without secrets) and a scheduled spec-drift check.

### Security

- HTTPS and public-destination defaults, origin-scoped exceptions, redirect credential scoping, URL redaction in errors, no way to disable certificate verification.
- Checksum `Verified` means every declared checksum was computed over the decoded artifact bytes and matched; it is integrity, not publisher authenticity. No signature is verified.
- The persistent cache is not a trust boundary (`docs/cache.md`).
- Release provenance: `docs/installing.md` says what each verification step proves and what it does not.

### Toolchain and dependencies

- `SKILLS.md`: a short guide for agents and scripts that call the CLI or library; `crates/tea-cli/tests/skills_doc.rs` checks its exit-status table against the code constants and `docs/cli-exit-codes.md`, and that every `libtea` subcommand and flag shown in the READMEs and `SKILLS.md` exists.
- MSRV 1.89. Policy: the latest stable release minus about four releases; raised only in minor releases, and every raise is listed here. Edition 2021.
- Dependencies use plain caret ranges and `Cargo.lock` is committed; `resolver = "3"`. `libtea-core` has no network, filesystem or async-runtime dependency; `libtea-client` uses `tokio` with its `fs` feature.

### Known limitations

- No cryptographic signature verification, publisher authenticity verification, or Sigstore trust-policy evaluation (`docs/design/signature-verification.md`).
- Live interoperability evidence is limited to a local server (Oolong) and one public read-only server (Vulnetix; one probe run is committed as `docs/interop/vulnetix-2026-11-08.json`, earlier results are reported only); not universal compatibility, not certification.
- No HTTPS SVCB lookup, OAuth beyond API-key `client_credentials`, full ECMA-428 CLE evaluation, `no_std`, or `Range` resume.
- The release workflow has run on GitHub (draft release, attestations verified with `gh attestation verify`); the Action has not yet been exercised from a workflow. Their third-party actions are pinned by commit SHA (`docs/release-pins.md`); older tag-pinned actions in `ci.yml` and two other workflows are listed there.
- CI runs on Linux only; macOS and Windows are not tested in CI, and the Unix-only directory checks of the cache do not exist on Windows.
- The cache's pruning rescans its directory, and streaming copies in flight are not counted against its total (`docs/cache.md`).

### Not public API (doctest-only items)

These exist only under `#[cfg(doctest)]` and are not part of any crate's public API: the private module `non_exhaustive_guards` in `libtea-core`, `libtea-client` and `libtea-conformance` (items `Guard…` and `Control…`, `pub(crate)`; each `#[non_exhaustive]` item has a `compile_fail` guard and a positive control, and `scripts/check-non-exhaustive-guards.sh` checks that every guard fails once the attributes are stripped), and `libtea_conformance::RootReadmeDoctests`.
