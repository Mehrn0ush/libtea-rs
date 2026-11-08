# 1.0.0 roadmap gap analysis (DevSecOps consumer)

> **Update 2026-11-08.** After this analysis was first written, these pre-0.1.0 changes landed in the working tree: `#[non_exhaustive]` on the wire model structs, tolerant discovery parsing (+ a warning-level strict lint in `libtea-conformance`), `ClientBuilder::lenient_versions`, a distinct `urn:tei:` error, re-exports of `url`/`semver`/`uuid`, and compiled README snippets. Rows below were updated; the findings on CLE, artifacts, trust, CLI, operations, interop, and quality gates are unchanged. See `CHANGELOG.md`.

**Date:** 2026-11-08 · **Tree:** uncommitted working tree on base `599e513` (after audit rounds 1–2) · **Method:** read-only. Source reading, `grep`, and the check results already recorded in `AUDIT.md`; no new builds, no network, nothing was run against a live server.

Status: **Present** (works and is tested), **Partial**, **Missing**, **Unverified** (cannot be shown from this repo). Evidence is `file:line` or a test name. Line numbers are for the working tree and will drift.

---

## 1. Spec versioning

| Finding | Status | Evidence |
| --- | --- | --- |
| Assumes exactly TEA OpenAPI 1.0.0 at commit `962a022…` | Present | `SupportedApi::tea_1_0_0()` `version.rs:66`; `spec::COMMIT`; `spec/PINNED.md` |
| Reads `endpoints[].versions` from `/.well-known/tea`, filters to supported, sorts by highest version then priority, tries candidates in order | Present | `select_endpoints` `version.rs:176-205`; `Client::discover` loop `lib.rs` (`for candidate in candidates`); `discovery_follows_version_and_stops_on_tea_404`, `prefers_higher_version_over_priority` |
| "Compatible" means **equal precedence to a listed version** by default; same-major only with `ClientBuilder::lenient_versions(true)` | Present (opt-in) | `SupportedApi::supports` (`version.rs`), `SupportedApi::with_lenient`; default off. Without the flag a server offering only `1.0.1` or `1.1.0` is rejected: `"no endpoint advertises an API version this client supports"`. With it, same-major versions are accepted (major 0 never). Tests: `same_major_is_selected_only_when_lenient`, `same_major_versions_need_the_flag`, `other_majors_are_rejected_even_when_lenient`. **Risk:** models were checked against 1.0.0 only |
| Server offering only an older version | Present (fails clearly) | Same error; surfaced from `discover` as `ClientError::Model(Invalid)` |
| Non-SemVer versions (e.g. Cybeats `["1"]`) | Present (rejected by default; accepted with `lenient_versions(true)`) | `ApiVersion::parse_lenient` reads `1` as `1.0.0` and keeps the spelling for the path (`/v1/`). Tests: `cybeats_shaped_short_version_is_rejected_by_default`, `cybeats_shaped_short_version_fails_by_default_and_works_when_lenient`. Not exercised against the live Cybeats server |
| Well-known / discovery JSON with unknown fields | Present (accepted) | `deny_unknown_fields` was removed from the four discovery wire types. Tests: `well_known_accepts_unknown_fields_at_every_level`, `discovery_accepts_unknown_fields_at_every_level`. The strict-schema check is a warning-level lint in `libtea-conformance` (`lint_well_known_strict`, `lint_discovery_strict`), never a client failure |
| `schemaVersion != 1` | Present (rejected) | `discovery.rs:21` |
| Resource models (Product, Artifact, Collection, Cle…) tolerate unknown fields | Present | no `deny_unknown_fields` in `model.rs`; optional fields use `#[serde(default)]`; the model structs are now `#[non_exhaustive]` |
| Missing required fields | Present (typed error) | serde errors → `ClientError::Json`; `empty_page_and_malformed_page_and_oversized_page_size` |
| Unknown enum strings preserved | Present | `wire_enum!` `Other(String)` (`enums.rs`); `preserves_unknown_artifact_type` |
| `#[non_exhaustive]` | Present for enums and config/outcome structs; **not** for wire model structs | `CHANGELOG.md` "Changed"; `Product`, `Artifact`, `Collection`, `Page`, … are exhaustive |
| Old `urn:tei:` input | Present (distinct error) | `TeiError::LegacyUrn`: "the legacy `urn:tei:` form is not supported; use `tei://…`" (case-insensitive prefix). Test: `legacy_urn_form_has_a_distinct_error` |

**What would break on a re-pin** (from `docs/re-pinning.md`, `version.rs`, `discovery.rs`, `checksum.rs`):
- Any new field on the four discovery wire types (hard fail).
- New required fields in resource schemas: tolerated on read only if serde-optional; otherwise typed `Json` errors.
- Adding variants to closed types: `ProductSort`, `ComponentSort`, `ReleaseSort`, `CollectionSort`, `SortOrder` are closed request enums (now `#[non_exhaustive]`, so additions are non-breaking for callers).
- New `checksum-type` values: `ChecksumAlgorithm::Other` is accepted but unverified → `Unsupported` (download errors).
- A new minor/major version: `tea_1_0_0()` and the path builder `versioned_api_prefix` (`/v{version}/`) assume that URL shape.
- Hand-written matrix and tests (`docs/api-operation-matrix.md`) have to be re-audited by hand; `spec-drift.yml` only diffs `openapi.yaml`, it does not generate or diff Rust.

## 2. TEI and discovery

| Finding | Status | Evidence |
| --- | --- | --- |
| Registered types validated: `uuid`, `purl`, `hash`, `eanupc`, `gtin`, `asin`, `udi`; unknown types parse as valid | Present | `Tei::validate_registry` `tei.rs:106-113`; `parses_registry_examples` |
| `swid` | Not a registered TEI type | `grep -rni swid spec/vendor` finds nothing (maintainer-verified 2026-11-08; the agent re-ran the check and the result is unchanged), so the pinned registry does not define it. Unknown types stay structurally valid and parse (`Tei::validate_registry` returns `Ok` for them); that behavior is kept on purpose |
| `purl` identifier checked as Base64URL, no padding, decodes to UTF-8, starts with `pkg:` | Present | `validate_purl` `tei.rs:247-262` |
| `urn:tei:` input | Rejected (see §1) | `TeiError::Scheme` |
| Multiple `.well-known` endpoints with fallback | Present | candidate loop in `Client::discover`; `non_tea_404_fails_over_to_the_next_endpoint`; a TEA 404 stops (`Unresolved`); 401/403 do not fail over |
| `.well-known` HTTPS-only | Present | `DestinationPolicy::check_url` rejects `http` unless that exact origin is excepted (`policy.rs:242-262`); well-known URL is `https://{domain}/.well-known/tea` (`Tei::well_known_url`) |
| HTTPS/SVCB DNS lookup | Missing | `docs/release-readiness.md:108`, `docs/roadmap.md:18`; system resolver only |
| Discovery attempt bound | Present | `Limits::max_discovery_attempts` (default 4) |

## 3. PURL

| Finding | Status | Evidence |
| --- | --- | --- |
| purl parsing / canonicalization / normalization (ECMA-427) | **Missing** | No `packageurl` (or similar) crate in `Cargo.toml` workspace deps; no purl type in `libtea-core`'s public API. `validate_purl` only checks the `pkg:` prefix after Base64URL decode |
| Official purl-spec test suite | Missing | no fixture or test references it (`grep` for `purl-spec` returns nothing) |
| purl as an *opaque filter value* | Present | `ListParams::with_id_type(Purl)` / `id_value`; `type_only_identifier_filter_is_serialized` |
| Consequence | | A caller who builds a TEI from a purl must canonicalize and Base64URL-encode it themselves; two spellings of one purl give two TEIs and no equality (`Tei` equality is by original spelling, deliberately, `does_not_treat_case_variants_as_equal`) |

## 4. CLE (ECMA-428)

| Finding | Status | Evidence |
| --- | --- | --- |
| Fetch `…/cle` for product, product release, component, component release | Present | `Service::get_*_cle` → `CleFetch::{Events, NotImplemented}`; `cle_success_for_every_cle_endpoint`; `cle_not_implemented_is_distinct_from_an_unknown_object` |
| Typed model of events, support definitions, version specifiers | Present | `Cle`, `CleEvent`, `CleDefinitions`, `CleSupportDefinition`, `CleVersionSpecifier` (`model.rs:411-495`) |
| Evaluate status for a component version (supported / EOL / unknown) | **Missing** | no function takes a version and returns a status; `grep` for lifecycle/status evaluators finds only the `CleFetch::Events` doc. `docs/roadmap.md` says "Client retrieves TEA CLE projection only" |
| ECMA-428 validation of the document | Missing | roadmap row "Full ECMA-428 CLE fetch/validate: 0.3+" |
| Public API for CLE | | `Service::{get_product_cle, get_product_release_cle, get_component_cle, get_component_release_cle}`; the CLI has **no** `cle` subcommand |
| Live evidence | Unverified | Oolong answers `NOT_IMPLEMENTED`; Vulnetix CLE not recorded (`docs/interoperability.md:75`). No server with real CLE data has been exercised |

## 5. VEX and artifacts

| Finding | Status | Evidence |
| --- | --- | --- |
| Get the artifacts of a release/collection | Present | `get_product_release` → `latest_collection.artifacts`; `get_*_collection`; `Collection.artifacts: Vec<Artifact>` (`model.rs:303`) |
| Select the VEX or SBOM artifact for a component version | **Partial** | `ArtifactType::{Bom, Vulnerabilities, …}` exists, and `Artifact::format(media_type)` picks a format (`model.rs:281`). There is no helper "give me the VEX for component X version Y": the caller walks release → collection → artifacts and filters by type. No CLI command does it (`download` takes an artifact UUID) |
| Streaming download with size limits | **Partial** | The body is read as a stream and both encoded and decoded size are capped *while reading* (`read_decoded`, `ContentDecoder`, `CappedBuf`; `redirect_limit_and_gzip_expansion_limit_are_enforced`), but the result is **buffered in a `Vec<u8>`** (`DownloadedArtifact.bytes`) |
| On-the-fly hashing | **Partial** | `ChecksumVerifier` is incremental (`checksum.rs`), but `Service::download_url` hashes the finished `Vec` with `verify_checksums_with_policy` (`service.rs:768`) |
| Resume (`Range`) | Missing | `grep` for `Range`/resume finds nothing |
| Safe write to disk | Partial | Library: none (returns bytes). CLI: create-new, `--force` via temp file + rename, symlink refusal (`output_file.rs`). Bytes are verified before the file is written; the whole artifact must fit in memory (default cap 256 MiB × `max_in_flight` 8) |
| Conditional download | Present | `if_none_match` → `ArtifactFetch::NotModified`; `conditional_get_304_is_not_a_checksum_and_has_no_body` |
| Lock / manifest of fetched artifacts (hash, server, spec commit, timestamp) | **Missing** | No type or file format. The probe script writes a result JSON with spec commit, server base URL, and timestamp, but it is an interop record, not a lock file, and it does not store artifact hashes |

## 6. Trust

| Question | Answer | Evidence |
| --- | --- | --- |
| What is verified? | Declared checksums match the **decoded** artifact bytes. `Verified` = every declared checksum computed and equal; SHA-1-only is `Weak`; unknown algorithms are `Unsupported` | `checksum.rs`; `weak_checksums.rs`; `docs/claims-audit.md` |
| Publisher authenticity? | **Not verified, anywhere.** The digest and the bytes come from the same server | `README.md` "Integrity versus authenticity"; `SECURITY.md` |
| TLS | Server identity of the transport only; certificate verification cannot be disabled; extra roots via `add_root_pem` | `transport.rs` `ClientBuilder::build` |
| `SignatureVerifier` today | A one-method trait, `verify(&[u8], &[u8], Option<&str>) -> SignatureVerifyOutcome`, with no implementation and **no call site**: the client never invokes it | `signature.rs:42`; its rustdoc says so |
| Real verifier / trust-anchor sourcing | Missing | `docs/design/signature-verification.md`: "0.1.0: no automatic trust-anchor fetch" |
| Signature bytes | Downloaded, not interpreted; no checksum exists for them in TEA | `SignatureFetch::Present`; `signature_bytes_are_not_reported_as_verified` |

**A 0.5 signature feature would have to change:**
1. The trait: add trust policy and expected signer identity as inputs (today only bytes); an outcome that separates *valid-but-untrusted* from *valid-and-trusted* (the design doc proposes `ValidAndTrusted`/`ValidButUntrusted`; current `SignatureVerifyOutcome` has `Verified`/`Invalid`/…, so either extend it — it is `#[non_exhaustive]` — or add a new type).
2. Digest binding: decide whether the signature covers the artifact bytes, the TEA checksum, or `Repr-Digest` (`docs/open-questions.md` #4).
3. `DownloadedArtifact`/`ArtifactFetch`: carry an optional verification result so callers get it with the download; today signature and artifact are separate calls with no linkage.
4. Dependencies and features: a verifier crate or feature (sigstore / OpenPGP / raw-key), `deny.toml` review, MSRV 1.89 compatibility check (`docs/design/signature-verification-deferred-notes.md`).
5. Trust-anchor sourcing: caller-supplied only (PEM/keyring trait); no fetch from the TEA host.
6. The lock file (§5): record signer identity and trust-policy ID next to the hash.
7. Test vectors listed in the design doc (good/untrusted/modified/wrong-signer/unsupported/missing roots/expired).
8. CLI: `--trust-root`, exit codes for invalid vs untrusted.

## 7. CLI for CI

| Finding | Status | Evidence |
| --- | --- | --- |
| Machine-readable output | **Partial** | `--json` on `spec`, `parse-tei`, `discover`, `product-release`, `collections` (6 flags in `main.rs`); `conformance` always prints JSON. `download` has no JSON status (stderr prose + bytes on stdout). The shapes are built ad hoc with `serde_json::json!`/`Serialize` of core models |
| JSON schema documented and tested | **Missing** | no schema file; no test asserts field names of CLI output (`grep` for "schema" in `crates/tea-cli` finds only a wiremock body) |
| Documented exit codes | **Partial** | 0 success, 1 any error, 4 weak-only checksums (`main.rs`, `weak_only_checksums_exit_4_unless_allowed`, `docs/interoperability.md`). Not in `--help` or the README; no code distinguishes auth failure, not found, or integrity failure |
| Non-interactive | Present | no prompts anywhere in `main.rs` |
| Credentials from environment | Present | `TEA_API_KEY_ID`/`TEA_API_KEY_SECRET`; precedence and partial-pair errors tested (`credentials.rs`, `resolve_tests`); `--api-key-secret` on argv is supported but documented as insecure |
| Log verbosity with secret redaction | **Partial** | The library emits `tracing::debug!` with `redact_url_for_log` (`transport.rs`), and errors strip URLs/queries (`url_redaction_tests`). The CLI has **no** `tracing` subscriber and no `-v/-q` flag (`tea-cli/Cargo.toml` has no tracing dependency), so those events are never shown |
| `sync` / `diff` | Missing | no such subcommand; subcommands are `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, `collections`, `download` |
| GitHub Action | Missing | no `action.yml` |
| Prebuilt binaries / container image | Missing | no release workflow, no `Dockerfile`; the CLI is `publish = false` |

## 8. Network and operations

| Capability | Status | Evidence |
| --- | --- | --- |
| Retry with backoff | Partial | `Limits::max_retries` (2) for GET transport failures, timeouts, and 503, exponential `initial_backoff` (`transport.rs:604-650`, `backoff`); `docs/security.md:63` |
| `Retry-After` | **Missing** | `grep -i retry-after` finds nothing in `crates/*/src` |
| 429 / rate-limit handling | Missing | no status 429 handling; 429 is returned as `ClientError::Api` |
| Timeouts | Present | `request_timeout` 30 s total, `connect_timeout` 10 s; token request bounded (`hanging_token_endpoint_times_out_without_the_secret`) |
| Proxy | Present | env proxies honored; `ignore_env_proxies`; DNS checks not enforced through a proxy (documented) |
| Host allowlist | Partial | `DestinationPolicy` is an *exception* list for HTTP/non-public origins on top of "public HTTPS". There is no "only these hosts" allowlist, so a hostile discovery document can still send the client to any public HTTPS host |
| Persistent cache + conditional requests | Missing / Partial | no cache (`README.md:215`); callers can pass `If-None-Match` per call, `ETag` is returned but not stored |
| Offline mode | Missing | |
| Concurrency limits | Present | `Limits::max_in_flight` semaphore (8) |
| Tracing / metrics hooks | Partial | `tracing` events (debug level, URLs redacted); no metrics |

## 9. API hygiene

**`libtea-core`** — `non_exhaustive`:
- Enums: `IntegrityOutcome`, `IpClass`, `EnumKnowledge`, `SortOrder`, `ProductSort`, `ComponentSort`, `ReleaseSort`, `CollectionSort`, all `wire_enum!` types (incl. `ChecksumAlgorithm`, `TeaErrorCode`, `ArtifactType`), `TeiError` (+ `Registry`), `ModelError`, `PolicyError`: **yes**.
- Structs: `ListParams`, `IntegrityReport`, `ChecksumCheck`, `ChecksumPolicy`: **yes**. **No**: `Checksum`, `Tei`, `Page`, `TeaErrorBody`, `TokenResponse`, `DestinationPolicy`, `ApiVersion`, `SupportedApi`, `SelectedEndpoint`, `VersionedEndpoint`, `WellKnownDocument`, `WellKnownEndpoint`, `DiscoveryInfo`, `DiscoveryServer`, `ChecksumVerifier`, `ContentDecoder`, and the model structs (`Product`, `ProductRelease`, `Component`, `ComponentRelease`, `Artifact`, `ArtifactFormat`, `Collection`, `CollectionUpdateReason`, `Identifier`, `Cle*`, `ReleaseDistribution`, …).

**`libtea-client`**: `non_exhaustive` — `ClientError` (and its struct variants `Api`, `Unresolved`, `DiscoveryFailed`, `Integrity`, `Token`), `ArtifactFetch`, `CleFetch`, `SignatureFetch` (+ struct variants), `PageFollowStop`, `SignatureVerifyOutcome`, `Limits`, `PageFollowLimits`, `FollowedPage`, `DownloadedArtifact`, `ResolvedRelease`. **No**: `Client`, `ClientBuilder`, `Service`, `ApiKey` (opaque, fine).

**Third-party types in the public API** (semver surface):

| Crate | Where | Risk |
| --- | --- | --- |
| `url::Url` | core: `DestinationPolicy::{allow_origin, and_allow, allows_origin, check_url, allows_resolved_ip}`, `origin_key`, `parse_api_base`, `with_query`; client: `ClientBuilder::well_known_fetch_url` | **Mitigated in 0.1.0:** re-exported as `libtea_core::url` / `libtea_client::url`; a major bump is still a breaking change. `url` is pinned `=2.5.4` for MSRV; a `url` major bump is a breaking change for this crate |
| `semver::Version` | core: `ApiVersion::semver()`, `SupportedApi::supports(&Version)` | `semver` 1.x | (re-exported as `libtea_core::semver`)
| `uuid::Uuid` | core: `TeaUuid::as_uuid()` | uuid: plain caret range (the MSRV 1.89 is set by uuid 1.29) | (re-exported as `libtea_core::uuid`)
| `serde` | every model derives `Serialize`/`Deserialize`; the serialized shape is public | any field rename is breaking |
| `std::net::IpAddr` | `classify_ip`, `allows_resolved_ip` | std, fine |
| `reqwest`, `serde_json`, `tokio`, `bytes`, `futures_util` | **not** in public signatures (errors carry `String`; `page_stream` returns `impl Stream`, which exposes the `futures_core::Stream` trait) | `follow_pages` makes `futures-core` Stream part of the surface |

**Semver risks:** (1) wire model structs were exhaustive with `pub` fields; they are now `#[non_exhaustive]` (2026-11-08), so adding a spec field is no longer breaking for callers, but a *removed or renamed* field still is; (2) `url`/`semver`/`uuid` types in signatures; (3) `ClientError` display text and `Api.message` are not contracts but are printed by the CLI; (4) `libtea-core` and `libtea-client` must move together (`libtea-core = "0.1.0"` workspace dependency); (5) `Tei` equality and `Display` semantics are load-bearing. `cargo semver-checks` is in CI but skips until a baseline exists on crates.io.

## 10. Interop evidence

| Server | How | Date | Recorded in | Reproducible by a third party? |
| --- | --- | --- | --- | --- |
| Oolong | live, local fixture, 10 `#[ignore]` tests | 2026-11-04 | `docs/interoperability.md` §"Checked-in live evidence", `oolong_idtype_live.rs`, `oolong_surface_live.rs` | Partly: needs the Oolong source (`node dist/main.js`) and its `fixtures/tea-1.0.0`, which are **not in this repo**; CLE and `/token` answer `NOT_IMPLEMENTED` there |
| Vulnetix | live, public, anonymous, read-only | not dated in the section I read (rows are "previously reported" and a probe) | `docs/interoperability.md:80-115` | **Yes**, `scripts/interop-probe.sh` with the env block in the doc. The probe's end-to-end behavior is tested only against local stubs (`interop_probe.rs`); I did not run it against Vulnetix |
| wiremock mocks | mock | every CI run | `crates/tea-client/tests/*`, `docs/api-operation-matrix.md` | Yes |
| Cybeats | one `.well-known` fetch | 2026-10-10 | `docs/interoperability.md:130` | No probe; the recorded `versions: ["1"]` would be rejected by this client |
| ReARM | — | — | "Not executed" | No |
| sbomify | — | — | "Not executed" | No |
| py-libtea, pypi-tea | — | — | "Not executed" | No |

Probe portability: `scripts/interop-probe.sh` takes `TEA_BASE_URL`/`TEA_TEI`/… and works against any server; `TEA_ALLOW_ORIGIN` allows a local HTTP server. `libtea-conformance` is **offline only** (`static_report()` has no networking, no URL parameter), so it cannot be pointed at a server. `interop-manual.yml` takes its target from the protected `tea-interop` Environment, which has not been created (maintainer step).

## 11. Quality gates

| Gate | In CI? | Evidence |
| --- | --- | --- |
| Property tests | Yes (run as ordinary tests) | `crates/tea-core/tests/properties.rs`, `parser_unicode.rs` (proptest) |
| Fuzzing | **No.** Six targets build locally (`cargo check --manifest-path fuzz/Cargo.toml`); ran for minutes by hand | no fuzz step in `.github/workflows/ci.yml` |
| `cargo-semver-checks` | Yes, skips per crate on crates.io 404 | `ci.yml` job `semver-checks`; **never exercised on GitHub** |
| `cargo-deny` | Yes (gating) | `ci.yml` `deny` |
| `cargo-audit` | Yes, `continue-on-error: true` | `ci.yml` `audit` |
| MSRV 1.89 | Yes (matrix `1.89.0`; tests, no clippy/fmt) | `ci.yml` `test` |
| Docs `-D warnings` | Yes | `ci.yml` "Documentation" step |
| Doctests | Yes | `ci.yml` "Doctests" step |
| Reproducible / signed releases | **No** | no release workflow; no `cargo-vet`, attestation, or checksum publication |
| SBOM of libtea-rs itself | **No** | `vulnetix-sbom.cdx.json` in the working directory is git-ignored and not part of any release |
| `SECURITY.md` | Present | reporting channel is GitHub Security Advisories; no response-time commitment |
| Dependabot / Renovate | No | no `.github/dependabot.yml` |
| Any workflow run on GitHub | **Unverified** | none has been run for this tree |

## 12. Docs and examples

| Finding | Status | Evidence |
| --- | --- | --- |
| End-to-end tutorial | Partial | `README.md` "Workflow A/B" snippets; `docs/architecture.md`. No step-by-step "discover → pick artifact → download → verify" tutorial |
| Examples that build | Present (4) | client `examples/filter_by_identifier_type.rs`, `paginate_components.rs`; core `parse_tei`, `select_endpoint`; built by `cargo test --all-targets` |
| Pipeline example (CI job, Dependency-Track, grype, osv-scanner) | **Missing** | no file; README mentions none |
| README claims backed by tests | Partial | Most behavioral claims map to named tests (`docs/api-operation-matrix.md`, `docs/claims-audit.md`). The Rust snippets in the workspace README (compiled from `libtea-conformance`, `RootReadmeDoctests`) and in both crate READMEs (included as crate docs) are now compiled as doctests; network snippets are `no_run`. No mechanical check ties README *prose* to tests. Live-compatibility statements are scoped to the two named deployments |

---

## Gates: status and shortest path to 1.0.0

Sizes: S ≤ 2 days, M ≈ 1–2 weeks, L > 2 weeks.

| Gate | Status now | Work items to close | Size |
| --- | --- | --- | --- |
| **Spec** | One pinned version; exact-version match; tolerant discovery JSON (2026-11-08), opt-in lenient versions | (a) decide same-major compatibility and a tolerant discovery mode (unknown fields, non-SemVer `"1"`); (b) tests for those; (c) clear `urn:tei:` error; (d) check `swid` against the TEI registry; (e) re-pin dry run to measure real breakage | M |
| **API** | `non_exhaustive` on enums/config; model structs exhaustive; `url`/`semver`/`uuid` exposed | (a) decide `non_exhaustive` for model structs or accept 0.x bumps until 1.0; (b) wrap or re-export `url`/`semver`/`uuid` types, or document them as semver surface; (c) publish 0.1.0 so `semver-checks` has a baseline; (d) freeze experimental `follow_pages` / `SignatureVerifier` or remove the label | M |
| **Interop** | 1 local fixture + 1 public server; 4 named servers never run | (a) probes for ReARM, sbomify, Cybeats with recorded dates and result files; (b) a live-mode conformance runner (conformance is offline-only); (c) a CLE-capable server in the evidence set; (d) fix Cybeats `"1"` handling or document the incompatibility | L |
| **Security** | Checksum integrity only; no authenticity; SSRF checks (not through proxies) | (a) real `SignatureVerifier` with caller-supplied trust (raw-key first); (b) host allowlist option; (c) proxy-address policy decision; (d) external review of `transport.rs`/`policy.rs`; (e) fuzzing in CI (scheduled job); (f) signed/reproducible release artifacts and an SBOM | L |
| **DevSecOps usability** | Library + CLI; no lock file, JSON schema, Action, binaries, cache | (a) documented, tested JSON output and exit codes; (b) CLI log levels with a subscriber; (c) lock file (hash, server, spec commit, time); (d) `sync`/`diff`; (e) `select VEX/SBOM for component version`; (f) streaming-to-disk download with resume; (g) GitHub Action, release binaries, container image; (h) one pipeline example (grype or Dependency-Track) | L |
| **Maintenance** | `SECURITY.md`, `cargo deny`, MSRV, MSRV 1.89, no dependency caps | (a) Dependabot/Renovate; (b) make `cargo audit` gating or document why not; (c) release workflow with provenance; (d) policy for MSRV and for spec re-pins; (e) run the CI workflows at least once on GitHub | M |

Smaller gaps on the way: `Retry-After`/429 handling (S), persistent cache with ETag (M), offline mode (S–M), purl handling (M; use a maintained crate and run the purl-spec suite), CLE evaluation (M), metrics hooks (S).

## Top 10 risks to a 1.0.0 date, in order

1. **No authenticity story.** Checksums come from the same server; DevSecOps buyers will read "verified" as "trusted". A real verifier needs design decisions that are still open (`docs/open-questions.md` #1–4) and a trust-anchor model TEA does not define.
2. **Interop evidence is two servers, one of them a local fixture.** Four named implementations have never been run, and the one non-SemVer server on record is rejected outright.
3. **Spec is pinned to one exact version** by default. Discovery now tolerates extra fields, and `lenient_versions(true)` accepts short and same-major versions, but the models were checked only against 1.0.0, so leniency trades a clear error for a possible misread.
4. **Public API not frozen.** Model structs are now `#[non_exhaustive]` and `url`/`semver`/`uuid` are re-exported; a major bump of any of them is still a breaking change, and `semver-checks` has never had a baseline.
5. **Nothing has run on GitHub.** CI, the new semver-checks logic, doctests, and the interop workflow are untested there; the Environment protecting the interop secret does not exist yet.
6. **CLI is not CI-grade.** No documented JSON schema or exit codes, no logging, no lock/sync, no Action or binaries; every DevSecOps adoption path depends on it.
7. **Missing purl and CLE logic** (ECMA-427/428). Both are hand-offs to the caller today; implementing them correctly needs the official test suites and a maintained crate.
8. **Memory-bound downloads** (whole artifact in a `Vec`, up to 256 MiB × 8 in flight) and no resume; large SBOM/VEX bundles on small CI runners are a real failure mode.
9. **SSRF controls have a documented hole behind proxies**, and there is no allowlist mode; CI runners commonly set proxy variables.
10. **Single maintainer, no external review, no scheduled fuzzing, no release signing.** Time-to-1.0 depends on review capacity as much as on code.

## Release train

Your suggestion: 0.2 CLI/lock/Action → 0.3 purl+discovery+cache → 0.4 CLE+VEX+adapters → 0.5 signatures → 0.6 interop+fuzz+review → 1.0.

Proposed, with the disagreements marked:

| Release | Content | Why |
| --- | --- | --- |
| **0.1.0** | Publish as is (after the handoff steps) | creates the `semver-checks` baseline and exercises CI; nothing below should be started on an unpublished API |
| **0.2** | Compat and API freeze work: tolerant discovery/version rules, `urn:tei:` message, third-party-type decision, `non_exhaustive` decision on models, `Retry-After`/429, tracing in the CLI. **Plus the first interop pass**: ReARM, sbomify, Cybeats probes | **Disagree:** interop moves from 0.6 to 0.2. Finding incompatibilities after the lock file, Action, and cache have been built on top of the current assumptions is the most expensive order. Interop results also decide the tolerance rules |
| **0.3** | CLI for CI: documented/tested JSON schema and exit codes, `download` JSON status, VEX/SBOM selection helper, streaming-to-disk download with incremental hashing and resume, lock file format, `sync`/`diff` (read-only) | **Agree** with CLI/lock first, but the lock file depends on streaming hashing and artifact selection, so those join it. **Defer the GitHub Action and binaries to 0.3 end**, once exit codes are stable |
| **0.4** | purl (maintained crate + purl-spec suite), persistent ETag cache and offline mode, SVCB (optional), host allowlist | **Disagree** with putting cache before the lock file: the cache design should follow the lock format. purl moves here because the lock and `sync` need a canonical identity |
| **0.5** | CLE evaluation + VEX/SBOM adapters (grype/osv-scanner/Dependency-Track examples) | **Disagree:** CLE and adapters before signatures, not after, because they do not depend on trust decisions. Signatures need the lock file format to be final |
| **0.6** | Signature verification (caller-supplied trust), recorded in the lock file | **Disagree:** this is the longest-lead item (open design questions, dependency review, MSRV). Start the *design decisions* (formats, trust anchors, digest binding) in 0.2 so code can start at 0.4 |
| **0.7** | Hardening: scheduled fuzzing in CI, external review of `transport.rs`/`policy.rs`/signature code, release signing + SBOM + provenance, Dependabot | **Disagree:** fuzzing in CI and Dependabot should start at 0.2 (S each); only the external review and signed releases belong at the end |
| **1.0** | API frozen after one full release cycle without breaking changes; interop table with ≥ 4 servers and dated result files | gate = items in the table above |

## Could not verify, and why

- **Any CI behavior on GitHub** (restrictions forbid triggering CI; nothing has been pushed).
- **Live-server behavior** (no network use): the Vulnetix and Cybeats observations are taken from `docs/interoperability.md`, and the probe was only run against local stubs. I did not find a date for the Vulnetix observations in the section I read.
- ~~Whether `swid` is a registered TEI type~~ — resolved: it is not (`grep -rni swid spec/vendor` finds nothing).
- **CLI `--help` text and the exact JSON field shapes**: read from `main.rs`, not by running the binary.
- **Memory/CPU behavior of large downloads** (not measured; the 2 GiB worst case is arithmetic from the defaults).
- **ReARM, sbomify, py-libtea, pypi-tea behavior**: no probe exists to run.
- **README claims line by line**: I mapped the major claims through `docs/claims-audit.md` and the matrix, not every sentence.
