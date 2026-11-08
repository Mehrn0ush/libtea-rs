# Release checklist (libtea-core → libtea-client)

Do **not** publish `libtea-cli` or `libtea-conformance` (`publish = false`).

A green **local** `cargo doc` is **not** sufficient. CI must be green on the
feature branch and again on `main` before anything is tagged or published.

## Exact order

1. **Record the reviewed SHA**: `git rev-parse HEAD` on the feature branch tip
   that was reviewed.
2. **CI green on the feature branch** at that SHA: `test` (stable and MSRV
   1.89.0, including the `cargo doc` step with `RUSTDOCFLAGS=-D warnings`),
   `package`, and `cargo deny` (which gates on RustSec advisories).
   `cargo audit` runs with `continue-on-error`; read its log, but it does not
   gate. `cargo semver-checks` skips each crate only while crates.io returns
   404 for it (first release). It fails on any other lookup error. From the
   release after 0.1.0 it runs against the latest published version; make it
   a required check then.
3. **Merge to `main` without squashing** (merge commit or fast-forward), so
   the reviewed commits, and the per-commit build proof from
   `HANDOFF-COMMANDS.md`, land unchanged.
4. **CI green on `main`** at the merge result. Record that SHA as
   `RELEASE_SHA`. If it differs from the reviewed SHA (a merge commit), the
   diff between them must be empty: `git diff <reviewed-sha> RELEASE_SHA`.
5. `cargo publish --dry-run -p libtea-core --locked` from a clean checkout of
   `RELEASE_SHA`.
6. **Tag `RELEASE_SHA`** (`v0.1.0`, annotated) and push the tag. The tag points
   at the `main` commit, never at a feature-branch commit.
7. `cargo publish -p libtea-core --locked`.
8. **Wait** until the index serves `libtea-core 0.1.0`, for example until
   `cargo info libtea-core@0.1.0` (or `cargo search libtea-core`) shows it.
9. `cargo publish --dry-run -p libtea-client --locked` (this resolves
   `libtea-core` from crates.io).
10. `cargo publish -p libtea-client --locked`.
11. **Post-publish checks:** a fresh project depending on both crates from
    crates.io builds and runs a `Tei::parse` / `Client::builder()` smoke;
    docs.rs pages for both crates build and show the independence /
    no-certification / checksum≠authenticity banners.

If step 5 or 9 fails, fix it on a branch and start again at step 1. Do not
move or re-point a pushed tag.

## Local preflight (does not replace CI)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo deny check
cargo audit
cargo package -p libtea-core --locked          # needs a clean tree
cargo package -p libtea-client --locked --no-verify --list
```

## Claims

State only what tests prove. Do not claim official conformance, universal
compatibility, or publisher-authenticity verification.

## Binary release (GitHub, never crates.io)

`.github/workflows/release.yml` builds the CLI for seven targets, writes
`SHA256SUMS` and a CycloneDX SBOM, attests build provenance, and creates a
**draft** GitHub release. It publishes nothing to crates.io and holds no
secret. It has **not been run**; the first run is the dry run below. Read
`docs/release-pins.md` first: four values are still placeholders.

1. Fill in the pins (`docs/release-pins.md`), then:
   `cargo test -p libtea-cli --test workflow_policy --test action_policy -- --ignored` must pass.
2. Dry run on a throwaway tag (a pre-release suffix makes it a draft
   prerelease and skips the "tag is on main" check):

   ```sh
   VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/tea-cli/Cargo.toml | head -n 1)
   git tag -a "v$VERSION-dryrun.1" -m "release workflow dry run"
   git push origin "v$VERSION-dryrun.1"
   gh run list --workflow release.yml --limit 3
   gh run watch                      # pick the run for the tag
   gh release view "v$VERSION-dryrun.1"
   gh release download "v$VERSION-dryrun.1" --dir /tmp/libtea-dryrun
   ```

   Then follow `docs/installing.md` on the downloaded files (checksums,
   `gh attestation verify`, run the binary on at least one platform you have,
   open the SBOM). Clean up:

   ```sh
   gh release delete "v$VERSION-dryrun.1" --yes --cleanup-tag
   git tag -d "v$VERSION-dryrun.1"
   ```

3. Real release: after step 10 of the crate release above, tag
   `RELEASE_SHA` (`v$VERSION`, annotated, on `main`); the workflow validates the
   tag against `crates/tea-cli/Cargo.toml`. Review the draft, check the files
   against `docs/installing.md`, then publish the release by hand.
