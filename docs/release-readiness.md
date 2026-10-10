# Release readiness

Date: 2026-10-10. Nothing was published. Crate names were not changed.

## Verdict

The 2026-10-10 audit blockers TEA-AUD-001 through TEA-AUD-005 and TEA-AUD-007 are fixed and covered by the regression tests named in `docs/audit-report.md`. The workspace is still not a conforming or certified TEA 1.0.0 consumer. It is implemented against CycloneDX Transparency Exchange API commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`. SDK version is `0.1.0`. Live interoperability was not run. Many list, release, and collection operations remain unverified. Publication is still blocked by crate names.

## Blockers before a conformance claim

The five items below were the audit blockers. They are fixed in this tree. A conformance claim still requires live interoperability and the unverified operations in `docs/api-operation-matrix.md`.

1. Decode HTTP `deflate` as zlib and test it. Fixed. `ContentDecoder` and `zlib_deflate_is_decoded_before_the_checksum`.
2. Make `HEAD` succeed when `Content-Length` describes the representation and the body is empty. Fixed. `head_ignores_representation_content_length`.
3. Scope DNS exceptions to the excepted origin, and test that. Fixed. `exception_is_scheme_host_and_port`.
4. Add the missing HTTP assertions for token stripping, discovery `403`, `304`, and pagination. Added in `crates/tea-client/tests/protocol.rs`.
5. Stop the CLI from saving unverified artifact bytes while claiming success. Fixed. Declared checksums are checked. `NotDeclared` is labeled as not verified. Stdout is bytes only.

## Blockers before crates.io

Checked with `GET https://crates.io/api/v1/crates/{name}` on 2026-10-10. A 404 means the name was absent at that moment. It can be registered later. A 200 means the name is taken by another project.

| Name | Registry result | This workspace |
| --- | --- | --- |
| `tea` | Taken. 0.1.0, "An environment for text-intensive interfaces" | Binary name in `tea-cli` is `tea` |
| `tea-core` | Taken. 0.5.1, "Core data structures and traits for tevec." | Library crate |
| `tea-cli` | Taken. 0.1.0, "Interactive and headless Coding CLI for tea-rs" | CLI package |
| `tea-rs` | Taken. 0.1.0, "Ergonomic embedding facade and product profile wiring for tea-rs" | Proposed repository name. Not a crate here |
| `tea-client` | HTTP 404 | Library crate |
| `tea-conformance` | HTTP 404 | Library crate |

`publish` is false on the workspace, which is appropriate. There is no `[package] repository`, `documentation`, `homepage`, or `readme` path on the member crates. The license field is `Apache-2.0` and `LICENSE` is the Apache 2.0 text. There are no feature flags. The public API is `0.1.0` and should be treated as unstable.

Do not rename crates in this audit. A later publish needs an explicit name that is still free at publish time. `tea-core` and `tea-cli` cannot be used as crates.io names while those packages exist. Publication order, if a new prefix is chosen, should be `tea-core`, then `tea-client` and `tea-conformance`, then the CLI, because the higher crates depend on `tea-core`.

## Tooling gaps

| Check | This machine on 2026-10-10, after remediation |
| --- | --- |
| fmt, clippy, test, doc | Passed on rustc 1.92.0 before the MSRV lockfile pins. `cargo +1.85.0 test --workspace --all-targets --all-features` then passed on 1.85.0 |
| `cargo audit` | Exit 0. Advisory database 1296 advisories. 223 crate dependencies. No advisory printed |
| `cargo deny check all` | Exit 0 after workspace path crates gained `version = "0.1.0"`. Advisories, bans, licenses, and sources are ok. Unmatched license allowances: BSD-2-Clause, MPL-2.0, OpenSSL, Unicode-DFS-2016. Duplicate warnings, left in place: `base64` 0.22.1 and 0.23.1, `cpufeatures` 0.2.17 and 0.3.1, `syn` 2.0.119 and 3.0.7, `windows-sys` 0.52.0 and 0.61.2 |
| Rust 1.85 | `cargo +1.85.0 check` and `cargo +1.85.0 test` passed. `rust-version` stayed `1.85`. The lockfile from 1.92 pulled `uuid` 1.28.0 (needs 1.89), `url` 2.5.8 with `idna` 1.1.0 and ICU 2.3 (needs 1.88), and `wiremock` 0.6.5 (let-chains, needs 1.88). Those are pinned to `uuid` 1.26.1, `url` 2.5.4, `idna` 1.0.3, `idna_adapter` 1.2.0, and `wiremock` 0.6.4. `.cargo/config.toml` sets `incompatible-rust-versions = "fallback"` so a later resolver prefers crates that declare a compatible rust-version. `wiremock` does not declare one, so it stays exact |
| Coverage | Lines 70.94% (945 of 3252 missed). Regions 71.20%. `service.rs` lines 50.83%. `transport.rs` 77.97%. `tea-cli` 68.80%. Previous audit: lines 56.95%, `service.rs` 25.04%, CLI 0% |
| Fuzz | `cargo +nightly fuzz build` then `cargo +nightly fuzz run <target> -- -max_total_time=5` for `tei_parse` (1,068,728 runs), `discovery_json` (385,498), `version_negotiation` (465,135), `artifact_metadata` (587,006), and `origin_policy`. Each exited 0. Nightly on this machine was 1.86. No crash artifacts |

Duplicate `synstructure` 0.13.2 and 0.14.0 appeared when ICU 1.5 was selected for the 1.85 graph. It is a warning. It was not deduplicated.

`Cargo.lock` is present and is not gitignored. That should stay.

## SemVer and API stability

Stay on 0.1.x until the download, HEAD, and policy defects are fixed. Those fixes can be made without a breaking change if `head_artifact`'s return type stays a header map and `deflate` only changes runtime behavior. Tightening `download_artifact` so an empty checksum list becomes an error would be a breaking change. Prefer a new method, or a 0.2.0 bump, if that default changes.

Edition 2024 is not a release requirement. The crate is edition 2021 with `rust-version` 1.85. Leave the edition alone unless a separate migration is requested.

## Documentation

`README.md` and `docs/` describe the pin, the limits, and the decision not to claim interoperability. This audit adds the five review documents and does not weaken those statements. `docs/requirements-matrix.md` still marks unverified rows as implemented. Readers should use `docs/conformance-gap-analysis.md` for status until that matrix is revised.

## Not a blocker

- Missing HTTPS SVCB lookup. The discovery text allows it when a client is known to support it. This client does not.
- Missing a persistent discovery cache. The text says implementations should respect caching headers. Callers can pass `If-None-Match` on downloads.
- Optional OAuth grants other than API-key `client_credentials`.
- Signature formats and `/trust-anchors/`. The signatures note is outside the standard, and that path is not in the pinned OpenAPI.
- Dropping alternate `servers[]` after the client has selected one. The spec requires a selection. Keeping the rest would be an API improvement, not a missing operation.
