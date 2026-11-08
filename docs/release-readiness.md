# Release readiness

Status for the first public release, 0.1.0. Package names are `libtea-core`, `libtea-client`, `libtea-conformance`, and `libtea-cli`; directory paths are `crates/tea-*`. Nothing has been published. Sections dated 2026-10-10 are the pre-release audit record and use the names of that time.

## Verdict

The 2026-10-10 audit blockers TEA-AUD-001 through TEA-AUD-005 and TEA-AUD-007 are fixed and covered by the regression tests named in `docs/audit-report.md`. The workspace is not a conforming or certified TEA 1.0.0 consumer. It is implemented against CycloneDX Transparency Exchange API commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`. SDK version is `0.1.0`. One local Oolong fixture profile has been exercised by checked-in ignored tests; that is recorded in `docs/interoperability.md` and is not a multi-implementation result. The names `tea-core` and `tea-cli` are taken on crates.io, so the workspace uses the `libtea-*` names recorded below. `libtea-cli` and `libtea-conformance` remain `publish = false`.

**Publication posture.** The 0.1.0 candidate is **not** published. It is published when the owner has committed a reviewed SHA, GitHub Actions is green on that SHA, the repository settings in `docs/release-checklist.md` are in place, `libtea-core` is published, then `libtea-client`. The CLI is released as binaries by the tag workflow (a draft the owner publishes by hand), which has not run on GitHub yet. Do not commit local download artifacts such as `vulnetix-sbom.cdx.json`.

## Blockers before a conformance claim

The five items below were the audit blockers. They are fixed in this tree. A conformance claim would still overreach: live evidence is one local Oolong server, and the unverified operations in `docs/api-operation-matrix.md` remain.

1. Decode HTTP `deflate` as zlib and test it. Fixed. `ContentDecoder` and `zlib_deflate_is_decoded_before_the_checksum`.
2. Make `HEAD` succeed when `Content-Length` describes the representation and the body is empty. Fixed. `head_ignores_representation_content_length`.
3. Scope DNS exceptions to the excepted origin, and test that. Fixed. `exception_is_scheme_host_and_port`.
4. Add the missing HTTP assertions for token stripping, discovery `403`, `304`, and pagination. Added in `crates/tea-client/tests/protocol.rs`.
5. Stop the CLI from saving unverified artifact bytes while claiming success. Fixed. Declared checksums are checked. `NotDeclared` is labeled as not verified. Stdout is bytes only.

## Publication names

Recheck with `GET https://crates.io/api/v1/crates/{name}` before publish. HTTP 404 means the name is absent at that request; it is not reserved.

| Package | Publish |
| --- | --- |
| `libtea-core` | `publish = true`. Publish this first |
| `libtea-client` | `publish = true`. Publish only after `libtea-core` 0.1.0 is on crates.io |
| `libtea-conformance` | `publish = false` |
| `libtea-cli` | `publish = false` |

`libtea-client` depends on `libtea-core` and does not depend on the CLI or the conformance crate. Workspace directories stay `crates/tea-core`, `crates/tea-client`, `crates/tea-conformance`, and `crates/tea-cli`. Rust import paths are `libtea_core` and `libtea_client`.

The CLI package is `libtea-cli` and the executable is `libtea` (not the colliding `tea` name). The CLI is not published to crates.io (`publish = false`); it ships as release binaries and via `cargo install --git`.

These libraries are an independent consumer implementation of the CycloneDX Transparency Exchange API. The `libtea` prefix does not make them a CycloneDX project release.

## Blockers before crates.io

Checked with `GET https://crates.io/api/v1/crates/{name}` on 2026-10-10, before the rename. A 404 means the name was absent at that moment. A 200 means the name was taken by another project. This table is the reason the old names were not published.

| Name | Registry result | This workspace |
| --- | --- | --- |
| `tea` | Taken. 0.1.0, "An environment for text-intensive interfaces" | Binary name in `tea-cli` is `tea` |
| `tea-core` | Taken. 0.5.1, "Core data structures and traits for tevec." | Library crate |
| `tea-cli` | Taken. 0.1.0, "Interactive and headless Coding CLI for tea-rs" | CLI package |
| `tea-rs` | Taken. 0.1.0, "Ergonomic embedding facade and product profile wiring for tea-rs" | Proposed repository name. Not a crate here |
| `tea-client` | HTTP 404 | Library crate |
| `tea-conformance` | HTTP 404 | Library crate |

`publish` is false on the workspace, which is appropriate. There is no `[package] repository`, `documentation`, `homepage`, or `readme` path on the member crates. The license field is `Apache-2.0` and `LICENSE` is the Apache 2.0 text. There are no feature flags. The public API is `0.1.0` and should be treated as unstable.

That 2026-10-10 note said not to rename during the audit. Names later moved from the taken `tea-*` crates.io names through a temporary `cdx-tea-*` prefix to the current `libtea-*` packages. `tea-core` and `tea-cli` still cannot be used as crates.io names while those other packages exist. Publish `libtea-core` 0.1.0, then `libtea-client` 0.1.0. Leave `libtea-conformance` and `libtea-cli` unpublished. The local executable is now `libtea`.

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

Those download, HEAD, and policy defects are fixed in this tree. `0.1.0` is still unpublished and the public API should be treated as unstable. Tightening `download_artifact` so an empty checksum list becomes an error would be a breaking change. Prefer a new method, or a 0.2.0 bump, if that default changes. The value-only identifier error text is `idValue requires idType`. That change does not change which inputs are rejected.

Edition 2024 is not a release requirement. The crate is edition 2021 with `rust-version` 1.89 (it was 1.85 until 2026-11-08). Leave the edition alone unless a separate migration is requested.

## Documentation

`README.md` and `docs/` describe the pin, the limits, and the single local Oolong result. They do not claim conformance. `docs/requirements-matrix.md` still marks some rows as implemented from code inspection. Readers should use `docs/api-operation-matrix.md` for which operations have a mock test, a live Oolong observation, or neither.

## 2026-11-04 release readiness update

Working tree on top of `b92faab` (dirty until committed). Changes in this pass:

- MSRV consumer contract (as of 2026-10-10; superseded on 2026-11-08, when the MSRV became 1.89 and the exact pins were removed): published `libtea-core` pinned `url = "=2.5.4"`, `idna = "=1.0.3"`, `idna_adapter = "=1.2.0"`, and `uuid = ">=1.26.0, <1.27.0"`. A fresh Rust 1.85 consumer of the packaged archive succeeds without this repository's `.cargo/config.toml`.
- Dependency advisories addressed in `Cargo.lock` (`rustls` 0.23.46, `rustls-webpki` 0.103.16, `webpki-roots` 1.0.11, `tokio` 1.54.1).
- Public error enums are `#[non_exhaustive]`.
- Transport logs and `ClientError::Transport` redact URL query values.
- `ListParams` docs describe safe pagination continuation.
- Live Oolong coverage expanded in `oolong_surface_live.rs` (see `docs/interoperability.md`).

Still required before publication (authorization gates):

1. Maintainer review and commit of this working tree.
2. Push and green GitHub Actions on that exact SHA.
3. Recheck crates.io names; publish `libtea-core` 0.1.0; wait for index; verify then publish `libtea-client` 0.1.0.
4. Fresh registry consumer smoke after both packages exist.

## 2026-10-12 release engineering

`cargo +1.85.0 test --workspace --all-targets --locked` passed on rustc 1.85.0, including the uncommitted authentication and deflate fixes. `cargo package -p libtea-core --allow-dirty --locked` verified the core archive. `cargo package -p libtea-client --allow-dirty --locked` failed because crates.io has no `libtea-core` 0.1.0 yet. `cargo package -p libtea-client --allow-dirty --locked --no-verify --list` listed the client files without that verification. The crates.io API returned HTTP 404 for `libtea-core` and `libtea-client` on this date. A 404 does not reserve either name.

## 2026-10-11 authentication regression

A stale `401` no longer deletes a newer bearer token for the same origin. The decoded-size cap is enforced inside the content decoder write. A truncated HTTP `deflate` member is rejected instead of returned as a short artifact. These are covered by local tests. Overlapping `POST /token` calls remain possible and are not treated as a release blocker. No hostile DNS server was used.

## Not a blocker

- Missing HTTPS SVCB lookup. The discovery text allows it when a client is known to support it. This client does not.
- The persistent cache is opt-in and experimental (`docs/cache.md`) and does not use `max-age`; every online use revalidates. Callers can pass `If-None-Match` on downloads.
- Optional OAuth grants other than API-key `client_credentials`.
- Signature formats and `/trust-anchors/`. The signatures note is outside the standard, and that path is not in the pinned OpenAPI.
- Dropping alternate `servers[]` after the client has selected one. The spec requires a selection. Keeping the rest would be an API improvement, not a missing operation.
