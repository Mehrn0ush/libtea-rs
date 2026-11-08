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

## Repository settings required before the first tag

The release workflow gives two jobs more than read access: `attest` (can sign
build provenance as this repository) and `draft-release` (can create a release).
Both run in the `release` Environment. What stops a stranger or a mistake is
configuration the workflow cannot enforce itself, so set it up first. Nothing
here is exercised by the test suite; check each item by hand.

1. **Environment `release`** (Settings, Environments). Required reviewers: at
   least one maintainer other than the person who pushes the tag, with "Prevent
   self-review" on. Deployment branches and tags: "Selected branches and tags",
   with the tag pattern `v*` only. Each privileged job asks for approval, so a
   release needs **two approvals** (one for `attest`, one for `draft-release`).
2. **Tag ruleset** (Settings, Rules, Rulesets, New tag ruleset). Target tags
   `v*`. Enforcement: Active. Rules: restrict creations, restrict updates,
   restrict deletions, block force pushes. Bypass list: only the maintainers who
   may release. Without it anyone with write access can create a `v*` tag.
3. **Branch protection for `main`** (a branch ruleset works too): require a pull
   request with at least one approval, require status checks (the CI jobs), block
   force pushes and deletion. The workflow refuses any tag whose commit is not on
   `main`; this makes "on main" mean "reviewed".
4. **Private vulnerability reporting** (Settings, Code security): enable it and
   make sure `SECURITY.md` says how to use it.
5. **Actions settings** (Settings, Actions, General): "Workflow permissions" set
   to read-only by default; "Allow GitHub Actions to create and approve pull
   requests" off. The release workflow declares its own permissions per job.
6. **Immutable releases** (Settings, General, if your plan offers it): turn it on
   so that a published release's assets cannot be replaced later. The Action's
   `SHA256SUMS` check is only as strong as the release's assets are fixed.

## Binary release (GitHub, never crates.io)

`.github/workflows/release.yml` builds the CLI for seven targets, writes
`SHA256SUMS` and a CycloneDX SBOM, attests build provenance, and creates a
**draft** GitHub release. It publishes nothing to crates.io, never publishes the
GitHub release itself, and holds no secret. It has **not been run**; the first
run is the dry run below. Read `docs/release-pins.md` first: four values are
still placeholders.

1. Fill in the pins (`docs/release-pins.md`), then:
   `cargo test -p libtea-cli --test workflow_policy --test action_policy -- --ignored` must pass.
2. Dry run. Every tag, dry runs included, must point at a commit on `main`; the
   workflow's first job fails otherwise. A tag with a suffix is a dry run: the
   release is a draft *prerelease* and the files are named with the whole tag
   version (`libtea-1.2.3-dryrun.1-...`). Tag a commit that is on `main`:

   ```sh
   git switch main && git pull --ff-only
   VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/tea-cli/Cargo.toml | head -n 1)
   git tag -a "v$VERSION-dryrun.1" -m "release workflow dry run"
   git push origin "v$VERSION-dryrun.1"
   gh run list --workflow release.yml --limit 3
   gh run watch                      # pick the run for the tag; approve the two deployments
   gh release view "v$VERSION-dryrun.1"
   gh release download "v$VERSION-dryrun.1" --dir /tmp/libtea-dryrun
   ```

   Then follow `docs/installing.md` on the downloaded files, with
   `VERSION=$VERSION-dryrun.1` (checksums, `gh attestation verify` with
   `--source-ref refs/tags/v$VERSION-dryrun.1`, run the binary on at least one
   platform you have, open the SBOM). Clean up:

   ```sh
   gh release delete "v$VERSION-dryrun.1" --yes --cleanup-tag
   git tag -d "v$VERSION-dryrun.1"
   ```

   The tag ruleset blocks deleting `v*` tags; the maintainer who may release
   deletes the dry-run tag through the bypass list, or the ruleset needs a
   second pattern that allows deleting `v*-dryrun.*`.

3. Real release: after step 10 of the crate release above, tag
   `RELEASE_SHA` (`v$VERSION`, annotated, on `main`); the workflow validates the
   tag against `crates/tea-cli/Cargo.toml`. Approve the two deployments. The
   result is a **draft** release. Review it, check the files against
   `docs/installing.md`, and publish the release by hand (the workflow never
   does).
