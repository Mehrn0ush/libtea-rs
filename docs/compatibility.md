# Compatibility

What this repository supports, and how each claim is checked. Last reviewed for
0.1.5 (2026-11-11).

## TEA specification

| | |
| --- | --- |
| Pinned contract | CycloneDX `transparency-exchange-api` commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`, OpenAPI `1.0.0` (`spec/PINNED.md`) |
| API versions accepted | Exactly `1.0.0` by default. In the library, `ClientBuilder::lenient_versions(true)` also accepts `1`, `1.0` and stable `1.x.y`; the CLI has no flag for it. The models were checked against the 1.0.0 schemas only |
| Not supported | TEA 0.x servers (for example 0.4.0). The legacy `urn:tei:` form is rejected with `TeiError::LegacyUrn` |
| Spec drift | `.github/workflows/spec-drift.yml`, weekly: diffs the pinned `openapi.yaml` against upstream `main` and uploads the diff. Informational, not a merge gate; other vendored files are not compared |

A server that works with this client is not, by that fact, a conforming TEA
server; see `docs/interoperability.md` for the servers it has been run against.

## Rust (library users)

| | |
| --- | --- |
| MSRV | 1.89, checked in CI (`rust 1.89.0` job) with the committed `Cargo.lock` |
| MSRV policy | Latest stable minus about four releases; raised only in a minor release, and listed in `CHANGELOG.md` |
| Edition | 2021 |
| Semver | `libtea-core` and `libtea-client` follow SemVer from 0.1.0, checked by `cargo semver-checks` in CI. Items marked **Experimental** (HTTP cache, lock file, signature extension point) may change in a 0.x minor release |
| `unsafe` | `#![forbid(unsafe_code)]` in every crate |
| TLS | rustls with bundled `webpki-roots`; no system certificate store, so trust roots change only with a new release |

## Platforms

| Target | Release archive | Release smoke test (`libtea spec --json`, `--help`) | CI test suite | GitHub Action run |
| --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | yes | yes | yes (`ubuntu-latest`, stable and 1.89.0) | yes (`ubuntu-24.04`) |
| `x86_64-unknown-linux-musl` | yes | yes | no | yes (`ubuntu-24.04`, `libc: musl`; `alpine:3.20` container) |
| `aarch64-unknown-linux-gnu` | yes | yes | no | yes (`ubuntu-24.04-arm`) |
| `aarch64-unknown-linux-musl` | yes | yes | no | no |
| `aarch64-apple-darwin` | yes | yes | yes (`macos-15`, stable) | yes (`macos-15`) |
| `x86_64-apple-darwin` | yes (cross-compiled) | **no** | no | no |
| `x86_64-pc-windows-msvc` | yes (`.zip`) | yes | yes (`windows-2022`, stable; Unix-only tests skipped) | yes (`windows-2022`) |

"GitHub Action run" means the Action installed that archive and ran
`parse-tei` from a workflow (scratch-repository run 42771741525 for 0.1.2;
`docs/github-action.md`). Commands that contact a server have not run through
the Action.

**Linux libc.** The gnu archives need glibc 2.34 or newer (Ubuntu 22.04+,
Debian 12+, RHEL 9+), measured on the v0.1.0 archives; they are built on
`ubuntu-24.04`. The musl archives are statically linked and run on older
distributions, Alpine and minimal containers. The Action does not detect the
distribution: set `libc: musl` where glibc is older or absent.

**Not signed.** Binaries are not code-signed or notarized; macOS Gatekeeper and
Windows SmartScreen may warn. Every archive is covered by `SHA256SUMS` and a
build-provenance attestation (`docs/installing.md`).

## Distribution channels

| Channel | Provenance |
| --- | --- |
| GitHub Releases archives | SHA-256 and attestation (`gh attestation verify`) |
| GitHub Action `Mehrn0ush/libtea-rs` (Marketplace: `libtea-rs`) | Downloads the archive, checks SHA-256, verifies the attestation when `gh` is present. Use 0.1.2 or later |
| crates.io (`libtea-core`, `libtea-client`) | crates.io checksums; no attestation |
| `cargo install --git ... --tag vX.Y.Z libtea-cli` | Builds from source; no checksum or attestation |

The CLI is not on crates.io (issue #14). There is no Homebrew formula or
container image (issue #17).
