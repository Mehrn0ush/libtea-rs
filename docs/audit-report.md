# Audit report

Audit date: 2026-10-10. Auditor role: independent review of the existing workspace. No implementation code was changed.

Later work fixed the defects named in this report and added mock tests plus one local Oolong run. Those results are in [interoperability.md](interoperability.md), [api-operation-matrix.md](api-operation-matrix.md), and [release-readiness.md](release-readiness.md). The findings below are the 2026-10-10 record. Where they say interoperability was not run, or that `deflate` and `HEAD` were incorrect, that describes this audit, not the tree after remediation. Package names in this record are the names from that date (`tea-core`, `tea-client`, `tea-conformance`, `tea-cli`). The current names are `cdx-tea-core`, `cdx-tea-client`, `cdx-tea-conformance`, and `cdx-tea-cli`. Directory paths did not change.

## Executive summary

The workspace is a real TEA consumer, not a stub. Every consumer operation in the pinned OpenAPI document has a method on `tea_client::Service`. TEI parsing, SemVer selection, discovery failover for a non-TEA 404, a TEA 404 stop, one bearer exchange, and SHA-256 mismatch handling are implemented and covered by tests.

That is not conformance. `cargo llvm-cov` on 2026-10-10 measured **56.95% line coverage** for the workspace and **25.04%** for `crates/tea-client/src/service.rs`. Most operations, `304`, `403`, `406`, pagination, `HEAD`, and `Content-Encoding: deflate` were not executed. The CLI has **0%** coverage.

| Question | Result |
| --- | --- |
| Mandatory consumer operations present in code | 28 OpenAPI operations, including `POST /token` |
| Mandatory consumer operations with an HTTP test | 4 paths: product GET, discovery GET, token POST, one artifact GET |
| Mandatory requirements missing as operations | None identified in the pinned consumer OpenAPI |
| Incorrect behavior | `deflate` decoding, `HEAD` body limit, DNS exception scope |
| Requirements not verified | See the gap analysis. Untested code is not counted as passing |
| Security findings | 1 high, 1 medium (unverified token-stripping test), 2 low |
| Interoperability | Not executed against a server. Public documents disagree with TEA 1.0.0 |
| Release blockers | Defects above, crate-name collisions, MSRV not run on this machine, `cargo deny` and `cargo fuzz` not run |

## Specification identity

Verified against GitHub, not against the current `main` branch.

| Claim | Evidence |
| --- | --- |
| Commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` | GitHub commit API. Committer date `2026-10-08T06:28:26Z`. Message: merge of PR 410. Signature `verified: true` |
| OpenAPI file at that commit | Blob `c96baa3e102c195895643fab81ae468ee54b8182`, 142993 bytes, SHA-256 `fc55bb5dacddfe63824a83e81b20e6dab31302553f3ff2d088689ff68b686812` |
| `openapi` | `3.1.1` |
| `info.version` | `1.0.0` |
| `info.title` | Transparency Exchange API |
| Example server | `http://localhost/tea/v1.0.0` |
| Spec README | Still says "Status of the standard: Beta 2" |

The SDK constants in `crates/tea-core/src/lib.rs` match this commit, version, and OpenAPI version. The retrieval date `2026-10-10` matches this audit's download. The earlier git clone of the repository failed with HTTP/2 `curl 92`; this audit used the contents API and raw file URLs at the pinned SHA.

## What was inspected

Cargo manifests, all Rust sources, tests, examples, fuzz targets, `deny.toml`, `.github/workflows/ci.yml`, and the existing docs. Normative inputs read at the pinned SHA: `spec/openapi.yaml`, `discovery/readme.md`, `auth/readme.md`, `tea-collection/tea-collection.md`, `doc/tea-implementations.md`, `discovery/tea-well-known.schema.json`, and the spec README.

## Architecture

The split is justified.

- `tea-core` has no Tokio, reqwest, or filesystem dependency. TEI, versions, models, checksums, and address classification can be tested without a network.
- `tea-client` owns HTTP, redirects, the token cache, and limits.
- `tea-conformance` is six static checks. It does not drive HTTP and does not certify a server.
- `tea-cli` calls the libraries. It also passes an empty checksum list into downloads, so it never verifies artifact bytes.

No new abstraction is recommended. The useful change is to make the existing download and policy paths correct.

`#![forbid(unsafe_code)]` is set on the libraries and the CLI. Unsafe code remains inside dependencies (`ring`, and others).

Edition 2024 is available on the declared MSRV of 1.85. This audit does not recommend changing the edition. Nothing found depends on an edition-2024 rule, and a migration would be a separate mechanical change.

## Tool results from this audit

Executed on rustc 1.92.0. All four requested Cargo commands exited 0.

| Tool | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --workspace --all-targets --all-features` | Pass. 8 HTTP tests, 23 `tea-core` unit tests, 3 property tests, 1 conformance test. CLI tests: 0 |
| `cargo doc --workspace --no-deps` | Pass, no rustdoc warnings in this run |
| `cargo audit`  | Exit 0. Advisory database loaded 1296 advisories. Scan reported 223 crate dependencies and printed no advisory |
| `cargo llvm-cov --workspace --all-features --summary-only` | Exit 0. Lines 56.95% (1329 of 3087 missed). Regions 61.37%. `service.rs` lines 25.04%. `tea-cli` lines 0%. Warning: "2 functions have mismatched data" |
| `cargo deny` | Not installed. `deny.toml` was not executed |
| `cargo fuzz` | Not installed. Targets under `fuzz/` were not executed |
| MSRV 1.85 | Not installed. `rustup toolchain list` showed stable 1.92, nightly, and 1.95. The GitHub Actions file names 1.85, and that workflow was not run |
| Live server tests | Not executed |

## Findings index

Severities and full write-ups are in `docs/security-review.md`, `docs/conformance-gap-analysis.md`, and `docs/interoperability-plan.md`.

| ID | Severity | Class | Status | Summary |
| --- | --- | --- | --- | --- |
| TEA-AUD-001 | High | Implementation | Verified | `Content-Encoding: deflate` was decoded as raw DEFLATE. It is now the zlib wrapper. Regression: `http_deflate_is_zlib_and_raw_deflate_is_rejected`, `zlib_deflate_is_decoded_before_the_checksum` |
| TEA-AUD-002 | High | Implementation | Verified | `head_artifact` treated `Content-Length` as a body. `HEAD` and `304` no longer do. Regression: `head_ignores_representation_content_length`, `conditional_get_304_is_not_a_checksum_and_has_no_body` |
| TEA-AUD-003 | High | Security | Verified | Origin exceptions were hostname-wide. They are scheme, host, and effective port, including at DNS time. Regression: `exception_is_scheme_host_and_port` |
| TEA-AUD-004 | Medium | Test gap | Verified | Redirect tests now install a real bearer token. Regression: `bearer_token_is_kept_under_the_prefix_and_stripped_after_that`, `head_redirect_uses_the_final_headers_and_strips_a_real_token` |
| TEA-AUD-005 | Medium | Implementation | Verified | The CLI fetches format metadata and verifies declared checksums. Status goes to stderr. Regression: `stdout_contains_only_verified_artifact_bytes`, `a_wrong_checksum_does_not_write_artifact_bytes` |
| TEA-AUD-006 | Medium | Architecture | Partially verified | Artifact bodies are still buffered up to the cap for hashing. Content coding is removed while reading, so the encoded copy is not kept beside the decoded copy. Caps: `redirect_limit_and_gzip_expansion_limit_are_enforced`, `gzip_expansion_is_capped` |
| TEA-AUD-007 | Low | Security | Verified | `0.0.0.0/8` and `198.18.0.0/15` are no longer public. Regression: `classifies_embedded_private_addresses` |
| TEA-AUD-008 | Low | API | Open | `well_known_fetch_url` can fetch discovery from a host other than the TEI domain. It remains a documented fixture hook |
| TEA-AUD-009 | Informational | Packaging | Open | `tea`, `tea-core`, `tea-cli`, and `tea-rs` already exist on crates.io. Names were not changed |
| TEA-AUD-010 | Informational | Interoperability | Open | Public servers and READMEs describe TEA 0.3 or 0.4, or advertise version `1`, which is not SemVer. Live tests were not run |

## Prioritized remediation

Do not start this work until it is authorized.

1. Fix `deflate` to use a zlib decoder, with a regression test of a zlib-wrapped body and a rejection of a truncated stream. Keep the decoded-size cap.
2. Fix `HEAD` so `Content-Length` of the selected representation is not treated as a body to download. Add a mock `HEAD` with `Content-Length` greater than zero and an empty body.
3. Make the DNS filter use the same origin key as `allows_origin` (scheme, host, and port). Add a test where only `http://name:8080` is excepted and `https://name` resolving to a loopback address is refused.
4. Re-test redirect token stripping with an API key and a cached bearer token. Assert the redirect target does not receive `Authorization`.
5. Stop the CLI from treating an empty checksum list as a successful download, and write artifact bytes only to `--output` or to stdout without a prefix line.
6. Add HTTP tests for `304`, `403` during discovery, pagination `pageToken`, and `406`. Do not mark those requirements passed before that.
7. Run `cargo deny`, install 1.85 and run the MSRV job, and run at least one fuzz target for a bounded time.
8. Leave crate names unchanged until a publish name is chosen. Do not publish.

## Remediation on 2026-10-10

The findings above were re-checked after the authorized fixes. Status in the index is the post-remediation state. The narrative in the sections above is the original audit and is kept.

Implemented against CycloneDX Transparency Exchange API commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`. SDK version `0.1.0`. This is not a conformance claim.

`tea-cli` no longer calls `download_artifact` with an empty checksum list. It loads artifact metadata and calls `download_format`. `IntegrityOutcome::Verified` prints `integrity verified` on stderr. `NotDeclared` prints that checksums were not declared and the bytes were not verified, then still writes the bytes, because the collection text requires checksums when a format `url` is present and allows an API-hosted format to omit them. `Failed`, `Unsupported`, and `Incomplete` fail before any artifact bytes are written. Stdout is the artifact only when `--output` is omitted.

Coverage after these tests, `cargo llvm-cov --workspace --all-features --summary-only`: lines 70.94% (945 of 3252 missed), regions 71.20%. `service.rs` lines 50.83% (was 25.04%). `tea-cli` lines 68.80% (was 0%). The same run still warns that 2 functions have mismatched data. Coverage is not a conformance result.

`cargo deny check all` passed (advisories, bans, licenses, sources). `cargo +1.85.0 test --workspace --all-targets --all-features` passed. `rust-version` is still `1.85`. Fuzz targets `tei_parse`, `discovery_json` (well-known, discovery list, and `tea_error_code`), `version_negotiation`, `artifact_metadata`, and `origin_policy` were built with `cargo +nightly fuzz build` and each ran under libFuzzer with `-max_total_time=5`. All five exited 0. No crash artifacts.
