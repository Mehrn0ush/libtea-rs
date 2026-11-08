# Changelog

All notable changes to this workspace are documented here. The publishable crates are `libtea-core` and `libtea-client`. Versions below refer to those crate versions unless noted.

## 0.1.0 — ready for first publish (not yet released to crates.io)

### Changed

- Repository renamed to `libtea-rs`. Publishable crate names are `libtea-core` and `libtea-client` (replacing the temporary `cdx-tea-*` package names).
- CLI package/binary aligned with the new names: package `libtea-cli`, executable `libtea` (was `tea`). Still `publish = false`.
- CLI download success line now states that “integrity verified” means decoded-byte checksums only (not publisher authenticity).
- CLI `download --require-checksums`: a format without declared checksums is an error, so exit status 0 means every declared checksum matched the decoded bytes. `scripts/interop-probe.sh` uses this instead of matching stderr text.

### Added

- Consumer TEA 1.0.0 client surface (`libtea-client::Service`) against pinned OpenAPI commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` (vendored under `spec/vendor/`; see `spec/PINNED.md`).
- TEI parsing, discovery, destination policy, checksum verification on decoded artifact bytes, API-key → bearer exchange.
- Token refresh **single-flight per API origin** on a `Client`. A same-origin burst of resource `401`s with no usable cached bearer costs **1** `POST /token`: waiters reuse the stored bearer after a success, and waiters that queued during a failed exchange return its failure. Different origins do not wait for each other. `POST /token` is bounded by `Limits::request_timeout`. See `docs/auth.md`.
- **Experimental** opt-in pagination in the new public module `libtea_client::page_stream`, re-exported at the crate root: `follow_pages`, `PageFollowLimits`, `PageFollowStop`, `FollowedPage` (may change in a 0.x minor). Caller-managed `ListParams` remains primary.
- **Experimental** `SignatureVerifier` / `SignatureVerifyOutcome` extension point in the new public module `libtea_client::signature`, re-exported at the crate root (may change in a 0.x minor). **No** default verifier returns `Verified`.
- `libtea_conformance::Report.is_certification` (always `false`).
- CLI (`libtea-cli`, unpublished): `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, `collections`, `download`.
- Credential environment variables `TEA_API_KEY_ID` / `TEA_API_KEY_SECRET`.
- Offline conformance smoke crate (`libtea-conformance`, unpublished); reports embed pinned commit.
- Interop probe script `scripts/interop-probe.sh`; optional manual CI workflow.
- Spec drift scheduled workflow (informational).

No existing `Service` / `Client` method signatures were changed relative to the pre-closeout surface (new modules/types only).

### Added and Changed (2026-11-08, maintainer decisions F4 and open question 14)

- **Added:** `libtea_core::escape_control_chars(&str) -> String`.
- **MSRV 1.89 for 0.1.0** (the first release, so there is no "raised" entry). Policy: the latest stable release minus about four releases; raised only in minor releases, and every raise is listed here. `rust-version = "1.89"` is set in the workspace and the fuzz crate; CI tests stable and 1.89.0.
- **Changed (dependencies):** no caps and no exact pins: `url`, `uuid`, and the other workspace dependencies use plain caret ranges, `wiremock = "0.6"`, the workspace uses `resolver = "3"`, and `Cargo.lock` was regenerated with `cargo update` (for example `uuid` 1.29.1, `idna_adapter` 1.2.2, `icu_*` 2.4.0, `proptest` 1.12.0, `wiremock` 0.6.5). `libtea-core` no longer declares the pin-only direct dependencies `idna` and `idna_adapter` (they remain transitive through `url`).
- **Changed (behavior):** server-supplied text is escaped when `ClientError::Api` (`tea_error`, `message`), `Token`, `Unresolved`, `Json`, and `Transport` values are built, and in the `Display` of `ModelError::{Json, Invalid}`: C0 (ESC, BEL, CR), DEL, C1 (including `U+009B` and `U+009D`), and bidi override/isolate characters become `\uXXXX`; tab and newline stay. Text without such characters is unchanged, so matching on ordinary TEA error codes still works. `libtea-cli` now reuses the core function.

### Not public API (doctest-only items)

These exist only under `#[cfg(doctest)]` and are not part of any crate's public API: the private module `non_exhaustive_guards` in `libtea-core`, `libtea-client`, and `libtea-conformance` (items `Guard…` / `Control…`, `pub(crate)`), and `libtea_conformance::RootReadmeDoctests` (a doctest host for the workspace README). Listed here so that a diff of `pub` items against the baseline is explainable.

### Added (2026-11-08 pre-0.1.0 changes)

- `libtea-core`: `TeiError::LegacyUrn` (returned by `Tei::parse` for `urn:tei:` input, any case); `ApiVersion::parse_lenient` (reads `1` and `1.2` as `1.0.0` / `1.2.0`, keeps the advertised spelling for the request path); `SupportedApi::{with_lenient, is_lenient}`; `WellKnownDocument::parse_lenient`; `DiscoveryInfo::parse_list_lenient`; `VersionedEndpoint::new`; `Page::new`; re-exports `libtea_core::{url, semver, uuid}`.
- `libtea-client`: `ClientBuilder::lenient_versions(bool)` (default `false`; when `true` accepts `1` / `1.0` and any *stable* version with the same major as a supported one (pre-releases only on an exact match); selection tries exact supported versions first, then same-major stable versions, highest first; major `0` is never matched by major alone; **risk:** the models were checked against the pinned 1.0.0 schemas only); re-export `libtea_client::url`.
- `libtea-conformance` (unpublished): `LintWarning`, `lint_well_known_strict`, `lint_discovery_strict`, `Report.warnings`. The strict-schema check reports fields the pinned schemas do not define as warnings and never fails a report.
- Crate documentation of `libtea-core` and `libtea-client` now includes the crate README, and the Rust snippets in both READMEs and in the workspace README are compiled as doctests (network snippets are `no_run`).
- New tests: `discovery_tolerance`, `lenient_versions` (core and client), `legacy_urn_and_reexports`, `strict_lint_*` (conformance), `compile_fail` guards for every `#[non_exhaustive]` item (66 guards, each with a positive-control doctest, in the private `#[cfg(doctest)]` module `non_exhaustive_guards` of each crate) and `scripts/check-non-exhaustive-guards.sh`, which strips every `#[non_exhaustive]` in a scratch copy and requires every guard to fail there, README doctests. An earlier version of these guards contained three that failed for the wrong reason; they were replaced (see `AUDIT.md`, Round 4).

### Changed (2026-11-08 pre-0.1.0 changes; breaking for 0.x)

- **`#[non_exhaustive]` on the wire model structs** of `libtea-core`: `Identifier`, `Product`, `ComponentRef`, `ProductRelease`, `ProductReleaseWithCollection`, `Component`, `ReleaseDistribution`, `ComponentRelease`, `ComponentReleaseWithCollection`, `CollectionUpdateReason`, `ArtifactFormat`, `Artifact`, `Collection`, `Page<T>`, `TeaErrorBody`, `TokenErrorBody`, `TokenResponse`, `CleSupportDefinition`, `CleDefinitions`, `CleEvent`, `CleVersionSpecifier`, `Cle`, plus `WellKnownEndpoint`, `DiscoveryInfo`, `DiscoveryServer`, `SelectedEndpoint`, `VersionedEndpoint`. Struct literals and exhaustive struct patterns for these no longer compile outside `libtea-core`; read them from JSON, use `Page::new` / `VersionedEndpoint::new`, and add `..` to patterns. `Checksum` already has private fields. `ComplianceDocument` (a one-field tuple struct) and the wrapper/ID newtypes are unchanged.
- **Discovery parsing is tolerant:** `deny_unknown_fields` was removed from the four discovery wire types, so a `.well-known/tea` or `/discovery` document with extra fields is accepted. Previously it failed with a JSON error. Required fields are still required.
- `libtea-conformance::Report` has a new public field `warnings` (the struct is `#[non_exhaustive]`).
- Default version handling is unchanged: without `lenient_versions(true)`, `1`, `1.0`, `1.0.1`, and `1.1.0` are still rejected.

### Added (2026-11-07 decisions, before first publish)

New public API:

- `libtea-core`: `IntegrityOutcome::Weak`; `ChecksumPolicy` (`allow_weak`, `new`, `with_allow_weak`); `verify_checksums_with_policy`; `ChecksumVerifier::with_policy`; `ChecksumCheck::weak`; `ChecksumAlgorithm::is_weak` (SHA-1, and an `MD5` label); `ChecksumCheck` re-exported at the crate root; `ListParams::with_page_size`, `with_page_token`, `with_sort_field`, `with_sort_order`, `with_id_type`, `with_id_value`.
- `libtea-client`: `ClientBuilder::allow_weak_checksums(bool)`; `ClientBuilder::ignore_env_proxies(bool)`; `Limits::with_max_discovery_bytes`, `with_max_json_bytes`, `with_max_artifact_bytes`, `with_max_redirects`, `with_request_timeout`, `with_connect_timeout`, `with_max_retries`, `with_max_discovery_attempts`, `with_initial_backoff`, `with_max_in_flight`; `PageFollowLimits::new`, `with_max_pages`, `with_max_items`; `ResolvedRelease::new`.
- CLI (unpublished): `download --force`, `download --allow-weak-checksums`; exit status 4 for a weak-only checksum match.
- Interop probe: `TEA_ALLOW_WEAK_CHECKSUMS=1`; `checksum_result: weak_algorithms_only`; `allow_weak_checksums` in results.

### Changed (2026-11-07 decisions, before first publish)

- **`#[non_exhaustive]`** added to:
  - `libtea-core` enums `IntegrityOutcome`, `IpClass`, `EnumKnowledge`, `SortOrder`, `ProductSort`, `ComponentSort`, `ReleaseSort`, `CollectionSort`, and every wire enum (`IdentifierType`, `ComplianceDocumentType`, `ArtifactType`, `CollectionUpdateReasonType`, `CollectionOwner`, `CleEventType`, `TeaErrorCode`, `ChecksumAlgorithm`); variant `TeiError::Registry`;
  - `libtea-core` structs `ListParams`, `IntegrityReport`, `ChecksumCheck`, `ChecksumPolicy`;
  - `libtea-client` enums `ArtifactFetch`, `CleFetch`, `SignatureFetch`, and their struct variants `ArtifactFetch::NotModified`, `CleFetch::NotImplemented`, `SignatureFetch::NotModified`; `ClientError` variants `Api`, `Unresolved`, `DiscoveryFailed`, `Integrity`, `Token`;
  - `libtea-client` structs `Limits`, `PageFollowLimits`, `FollowedPage`, `DownloadedArtifact`, `ResolvedRelease`;
  - `libtea-conformance` structs `Report`, `CaseResult` (unpublished).

  Already `#[non_exhaustive]`: `TeiError`, `ModelError`, `PolicyError`, `ClientError`, `PageFollowStop`, `SignatureVerifyOutcome`. The wire model structs were made `#[non_exhaustive]` on 2026-11-08 (see above). Struct literals such as `Limits { .., ..Limits::default() }` and `ListParams { .., ..ListParams::default() }` no longer compile; use `default()` plus `with_*` setters, or assign fields on a default value.
- A checksum match on weak algorithms only (SHA-1) is `IntegrityOutcome::Weak`, not `Verified`; `Service::download_*` returns `ClientError::Integrity` for it unless `allow_weak_checksums(true)`.
- Environment proxies are still honored by default; the docs now state that DNS-based destination checks do not apply through a proxy.
- CLI `download --output` creates a new file and refuses an existing path (unless `--force`) and a symbolic link as the **final path component** (always). Parent directories are not checked and may be symlinks (for example `/tmp` on macOS); the write goes through them, and a test records that as intended.
- Per-origin token-refresh lock entries are removed by their last holder, including on cancellation, so the map does not grow with the number of origins.
- `.github/workflows/interop-manual.yml` takes every target from the protected `tea-interop` Environment, has no URL input, and runs with `permissions: {}` at the top level.
- CI runs `cargo test --workspace --doc` (the `compile_fail` guards) and `scripts/check-non-exhaustive-guards.sh` (the mutation check for them).

### Fixed (2026-11-08 review, before first publish)

- `.github/workflows/interop-manual.yml` ran `cargo build` (third-party build scripts) in the step that held `TEA_API_KEY_*`. The CLI is now built in a step without secrets; the key reaches one step that runs the prebuilt binary. `scripts/interop-probe.sh` refuses to run `cargo` when credentials are set and `TEA_CLI_BIN` is not, reads the client version from the manifest instead of `cargo metadata`, and no longer calls `cargo` when `TEA_CLI_BIN` is set.
- `lenient_versions(true)` preferred the highest advertised version over an exact match and accepted pre-releases of the same major. Exact supported versions now sort first; only stable same-major versions are accepted otherwise. Strict mode is unchanged.
- The CLI printed server-supplied strings (token error text, API error codes, page tokens) with control characters intact. Every line the CLI prints is now passed through an escaper (`\uXXXX` for C0, DEL, C1, and bidi override characters; tab and newline are kept). JSON output stays valid.
- Timing assertions in the token-refresh tests were tight wall-clock budgets; they are now causal or relative to the configured timeout.
- Three `compile_fail` guards (`IntegrityOutcome`, `ChecksumCheck`, `FollowedPage`) failed for reasons other than `#[non_exhaustive]`. All guards were regenerated with controls and a mutation check.
- The docs overstated symbolic-link handling of `download --output` (only the final path component is checked) and the Environment guarantee of the interop workflow.

### Fixed (2026-11-07 audit, before first publish)

- `Tei::parse` panicked on a non-ASCII character in the two bytes after `%` (for example `tei://example.com/uuid/%aé`). Percent escapes are now checked byte-wise.
- `classify_ip` treated IPv4-compatible IPv6 (`::a.b.c.d`), site-local `fec0::/10`, and NAT64 local-use `64:ff9b:1::/48` as public. They now take the embedded IPv4 class or `Private`.
- `follow_pages` followed an empty `next_page_token` (re-requesting the first page). It now stops with `PageFollowStop::Exhausted`, as documented.
- Token refresh used one client-wide lock held across `POST /token`: a hanging token endpoint on one origin stalled refreshes for every other origin, and each queued waiter repeated the full timeout.
- `scripts/interop-probe.sh`: secret redaction treated the secret as a glob pattern; a run with nothing to probe exited 0; `spec --json` failures were hidden; the script could `chmod` a caller-supplied directory; result JSON was built by string interpolation.

### Security

- HTTPS / public-destination defaults; origin-scoped exceptions; redirect credential scoping; transport URL redaction in errors.
- Checksum `Verified` only when every declared algorithm is implemented and matches the decoded artifact bytes; it does not imply publisher authenticity.
- TLS certificate verification cannot be disabled.

### Known limitations

- No cryptographic signature verification, publisher authenticity verification, or Sigstore trust-policy evaluation. See `docs/design/signature-verification.md`.
- Live interoperability evidence is Oolong (local) and Vulnetix (public read-only); not universal compatibility.
- CLI and conformance remain unpublished (`publish = false`).
- No HTTPS SVCB, persistent HTTP cache, OAuth beyond API-key `client_credentials`, full ECMA-428 CLE validate, or `no_std` (see `docs/roadmap.md`).

### Added (F1: streaming download, `--json`, exit codes; unreleased)

- **Experimental** streaming download in `libtea-client`: `Service::download_format_to_file`, `download_artifact_to_file`, `download_latest_artifact_to_file` (temporary file, renamed only after the stream ended cleanly and the checksum policy passed; removed on failure or cancellation; symlink destinations refused; `FileDownloadOptions::overwrite`) and `download_format_to_writer` (the writer sees unverified bytes until the call returns `Ok`). New types `StreamedDownload`, `FileDownload`, `FileDownloadOptions` (all `#[non_exhaustive]`). New `ClientError::Io` variant. Size, decode, redirect and destination limits are the same as for the whole-body methods. Range resume is not supported.
- `libtea_core::checksum::ContentDecoder::take_output` (experimental): hand out decoded bytes as they are produced; the size cap still covers the whole body.
- CLI: `download --output` now streams; `--json` on every command (`conformance` and `download` gained it); `download --json` requires `--output` and prints one object on stdout; with `--json` a failure is one JSON object on stderr.
- CLI exit codes: 2 usage, 3 auth, 5 not found, 6 integrity (4 weak-only and 1 generic are unchanged; 7 is reserved for changes detected). Documented in `docs/cli-exit-codes.md`, `docs/cli-json.md` and `--help`. **Behavior change:** failures that exited 1 before (a bad TEI, a missing credential, a 404, a checksum mismatch) now exit 2, 3, 5 or 6. A script that tests for exactly 1 must be updated.
- `tokio` `fs` feature enabled; no new crate.

### Added (F2: lock file, sync, diff; unreleased)

- **Experimental** lock file in `libtea-core` (`libtea_core::lock`, re-exported: `LockFile`, `LockedArtifact`, `LockedChecksum`, `LockedIntegrity`, `LockDiff`, `FieldChange`, `ChangedArtifact`, `diff_locks`, `LOCK_SCHEMA_VERSION`): `schemaVersion` 1, strict parsing, canonical deterministic serialisation, validation (no secrets in `server`, no duplicate entries), and a diff that ignores `fetchedAt`. Documented in `docs/lock-file.md` with `docs/schemas/lock-v1.schema.json`.
- CLI: `libtea sync <TEI> --lock FILE [--check] [--artifacts-dir DIR]` (resolves, downloads and verifies every artifact format of the latest collection, writes the lock atomically, leaves an unchanged lock alone, fails closed) and `libtea diff OLD NEW`. Exit status 7 now means "changes detected" (`sync --check`, `diff`). `--json` output is described in `docs/cli-json.md`.
- New fuzz target `lock_parse`.

### Added (F3: release workflow; unreleased)

- `.github/workflows/release.yml`: tag-triggered (`v*` tags only) build of the CLI for Linux x86_64 and aarch64 (gnu and musl), macOS x86_64 and aarch64, and Windows x86_64; `SHA256SUMS`; a CycloneDX SBOM of the CLI's dependency tree; a GitHub build-provenance attestation; a **draft** release. It never publishes to crates.io and holds no repository secret; each job has its own least-privilege `permissions`. **Not run yet**, and four pins are placeholders (`docs/release-pins.md`).
- `scripts/package-release.py` (deterministic archives), `scripts/check-release-tag.sh` (tag validation), `scripts/make-sbom.sh`; tests in `crates/tea-cli/tests/release_scripts.rs` and the extended `workflow_policy.rs`.
- Docs: `docs/installing.md`, `docs/release-pins.md`, a binary-release section and dry-run commands in `docs/release-checklist.md`.

### Added (F4: GitHub Action; unreleased)

- `action.yml` (composite): installs a released `libtea` binary for the runner, checks its SHA-256 against the release's `SHA256SUMS` (fetched on every run; a cached archive must match too), verifies the build-provenance attestation when `gh` is available, caches the archive, and runs `sync` (with `check`), `diff`, `discover`, `download`, `product-release`, `collections`, `parse-tei`, `spec` or `conformance`. No credential input: `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET` come from the caller's `env:`. Scripts in `scripts/action/`; guide and example workflows (including a `sync --check` job) in `docs/github-action.md`. Not run on GitHub yet; `actions/cache` is a placeholder pin (`docs/release-pins.md`).

### Added (F6: cache, offline mode, Retry-After; unreleased)

- **Experimental** opt-in persistent cache: `ClientBuilder::cache_dir`, `ClientBuilder::cache(CacheConfig)`, `ClientBuilder::offline`, `libtea_client::{CacheConfig, CacheStats, cache_stat, cache_clear}`. Conditional `GET`s with `ETag` / `Last-Modified`; keyed by method, normalized URL and a hash of origin and key id (never the secret); honors `Cache-Control: no-store` and (by default) `private`; responses to credentialed requests are not stored unless enabled; private directory (0700 / 0600, symlink and open directories refused); atomic writes; total-size, entry-size and age limits; damaged entries are treated as absent; artifact checksums are re-verified on every use. Streaming downloads are cached after they verify. See `docs/cache.md`.
- CLI: `--cache-dir` (or `LIBTEA_CACHE_DIR`), `--offline`, `--cache-authenticated` on `discover`, `product-release`, `collections`, `download`, `sync`; `libtea cache stat|clear`.
- `libtea_core::http_cache` (experimental): cache key, `Cache-Control` and `Retry-After` parsers, storage decision, entry codec. Proptests and the `http_cache_entry` and `http_cache_headers` fuzz targets.
- `Limits::max_retry_after` (default 30 s) and `ClientError::Throttled { status, retry_after_secs }`: `GET`s answered `429` or `503` honor `Retry-After` (seconds or HTTP-date) up to the cap and fail at once above it. **Behavior change:** a final `503` is now `Throttled`, not `Transport("HTTP 503")`. `ClientError::Offline` is new.
