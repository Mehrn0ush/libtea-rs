# Folding `libtea-conformance` into the CLI and publishing `libtea-cli` 0.2: plan

**Status:** plan for issue #14, for review. Nothing here is done.

## Why

A crate on crates.io cannot depend on an unpublished one, and `libtea-cli`
depends on `libtea-conformance` for one call: `libtea_conformance::static_report()`
in `crates/tea-cli/src/main.rs` (the `conformance` command). Publishing the CLI
lets users run `cargo install libtea-cli`, and gives `cargo-binstall` a source.

## Names on crates.io (checked 2026-11-11, `GET /api/v1/crates/<name>`)

`libtea-cli`: 404 (free). `libtea`: 404 (free). `libtea-conformance`: 404.

`libtea` is the binary's name and would read best, but a crate called `libtea`
that is a CLI, next to libraries called `libtea-core` and `libtea-client`, will
be mistaken for the main library. Recommendation: publish as `libtea-cli`
(binary still `libtea`), and do not register `libtea` unless you want to hold
the name.

## What moves where

| From | To |
| --- | --- |
| `crates/tea-conformance/src/lib.rs` (cases, lints, `Report`) | `crates/tea-cli/src/conformance.rs`, `pub(crate)` |
| `crates/tea-conformance/src/non_exhaustive_guards.rs` | `crates/tea-cli/src/` if the types stay `#[non_exhaustive]`; dropped if they become `pub(crate)` |
| `crates/tea-conformance/tests/report.rs` (added in #29) | `crates/tea-cli/tests/conformance_report.rs`, through the CLI's `conformance --json` output instead of the library API |
| `RootReadmeDoctests` (compiles the root `README.md` snippets) | A doctest host in `libtea-client` (it already depends on core and has tokio as a dev-dependency); `#[cfg(doctest)]` keeps it out of the published API. `#[doc = include_str!("../../../README.md")]` points outside the crate directory, which breaks `cargo package` for a published crate, so the README must be copied or the host must stay in an unpublished crate (see decision 2). |

Then remove `crates/tea-conformance`, its workspace entry and the
`[workspace.dependencies]` line.

## The CLI's public surface after publishing

A binary crate has no Rust API to keep stable. What users depend on is:

- command names, flags and exit codes (`docs/cli-exit-codes.md`);
- the `--json` shapes (`docs/cli-json.md`), including `conformance --json`;
- the lock-file format (`docs/lock-file.md`, `docs/schemas/lock-v1.schema.json`).

These are already documented and tested (`json_and_exit_codes.rs`,
`lock_schema.rs`). Publishing makes them a semver promise: a breaking change to
any of them is a new minor version while 0.x.

## Versioning

- `libtea-cli` 0.2.0: the first published version. Using 0.2 rather than 0.1.x
  marks the change of distribution and lets the CLI version diverge from the
  libraries if needed later.
- `libtea-core` / `libtea-client` are unaffected by the move and stay on their
  own line (0.1.x) unless something else changes them. Today all crates share
  one version; this plan ends that. The release checklist, `scripts/` and the
  release workflow assume one version for the tag (`v0.1.5` names everything);
  they need a decision (decision 3).

## Steps (one PR each)

1. Move the code, keep `libtea-conformance` as an empty shim that re-exports
   nothing, run the full gate. No behavior change; `conformance --json` output
   byte-identical (test compares with a fixture taken before the move).
2. Move the README doctest host. Remove the shim crate.
3. Lift `publish = false` from `libtea-cli`, add `readme`, `keywords`,
   `categories`, `documentation`; `cargo package -p libtea-cli --list` reviewed
   for stray files (fixtures, scripts). Add `cargo package -p libtea-cli` to the
   CI `package` job.
4. Docs: `docs/installing.md` (crates.io route, which, unlike the release
   archives, has no attestation: say so), `README.md`, `docs/architecture.md`,
   `docs/release-checklist.md` (publish order core → client → cli), and the
   section "How the CLI is distributed" in the checklist (which mentions CI path
   filters; there are none, so drop that).
5. Release prep PR for the version(s).

## Decisions for you

1. Crate name: `libtea-cli` (recommended) or `libtea`.
2. README doctests: move the host into `libtea-client` with a copied README
   (simple, but two READMEs to keep in sync), or keep a tiny unpublished crate
   only for doctests (no duplication; the workspace keeps one `publish = false`
   member, which is harmless once nothing published depends on it).
   Recommendation: keep a tiny unpublished `doctests` crate.
3. One version for everything (CLI 0.2.0 forces the libraries to 0.2.0, a
   no-change major-ish bump for them) or independent versions (tags then need a
   crate prefix, for example `cli-v0.2.0`, and the release workflow and
   `check-release-tag.sh` change). Recommendation: one version, bump everything
   to 0.2.0 together, because the release tooling assumes it and the libraries
   are pre-1.0 anyway; say clearly in the changelog that 0.2.0 has no library
   change.
