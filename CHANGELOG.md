# Changelog

All notable changes to this workspace are documented here. The publishable crates are `libtea-core` and `libtea-client`; `libtea-cli` and `libtea-conformance` are not published to crates.io (`publish = false`). Versions refer to the publishable crates unless noted.

Items marked **Experimental** may change in a minor release before 0.9; everything else follows semantic versioning from this release on.

## 0.1.0 — first release (not yet published)

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
- Scripts: `scripts/package-release.py`, `scripts/check-release-tag.sh`, `scripts/check-pins.sh`, `scripts/make-sbom.sh`, `scripts/action/*`, `scripts/interop-probe.sh`, `scripts/check-non-exhaustive-guards.sh`.
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
- Live interoperability evidence is limited to a local server (Oolong) and one public read-only server (Vulnetix; reported by the maintainer, no result file committed); not universal compatibility, not certification.
- No HTTPS SVCB lookup, OAuth beyond API-key `client_credentials`, full ECMA-428 CLE evaluation, `no_std`, or `Range` resume.
- Not yet run on GitHub: the release workflow and the Action. Their third-party actions are pinned by commit SHA (`docs/release-pins.md`); older tag-pinned actions in `ci.yml` and two other workflows are listed there.
- CI runs on Linux only; macOS and Windows are not tested in CI, and the Unix-only directory checks of the cache do not exist on Windows.
- The cache's pruning rescans its directory, and streaming copies in flight are not counted against its total (`docs/cache.md`).

### Not public API (doctest-only items)

These exist only under `#[cfg(doctest)]` and are not part of any crate's public API: the private module `non_exhaustive_guards` in `libtea-core`, `libtea-client` and `libtea-conformance` (items `Guard…` and `Control…`, `pub(crate)`; each `#[non_exhaustive]` item has a `compile_fail` guard and a positive control, and `scripts/check-non-exhaustive-guards.sh` checks that every guard fails once the attributes are stripped), and `libtea_conformance::RootReadmeDoctests`.
