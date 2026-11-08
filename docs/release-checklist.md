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
6. **Tag `RELEASE_SHA`** once: `git tag -a v0.1.0 RELEASE_SHA -m "libtea 0.1.0"`
   and push the tag. The tag points at the `main` commit, never at a
   feature-branch commit. This is the only tag; there is no separate trial tag.
7. **Binary release workflow** (details in "Binary release" below). The tag
   starts `release.yml`. Approve the two deployments (`attest`,
   `draft-release`), download the **draft** release, and verify it with
   `docs/installing.md`: checksums, `gh attestation verify` with
   `--source-ref refs/tags/v$VERSION`, run the binary, open the SBOM. Do not go
   on until this passes. Nothing has been published to crates.io yet.
8. `cargo publish -p libtea-core --locked`.
9. **Wait** until the index serves `libtea-core 0.1.0`, for example until
   `cargo info libtea-core@0.1.0` (or `cargo search libtea-core`) shows it.
10. `cargo publish --dry-run -p libtea-client --locked` (this resolves
    `libtea-core` from crates.io).
11. `cargo publish -p libtea-client --locked`.
12. **Publish the GitHub draft: publish the release by hand** (the workflow
    never does).
13. **Post-publish checks:** a fresh project depending on both crates from
    crates.io builds and runs a `Tei::parse` / `Client::builder()` smoke;
    docs.rs pages for both crates build and show the independence /
    no-certification / checksum≠authenticity banners.

**If something fails.** Before the first `cargo publish` (steps 5 to 7: the
workflow, or the verification of the draft): the maintainer deletes the draft
release and the tag (`gh release delete v0.1.0 --yes --cleanup-tag`, which needs
the tag ruleset's bypass list), fixes the problem on a branch, merges it to
`main` by pull request, records the new `RELEASE_SHA`, and tags that commit.
After the first `cargo publish` succeeded, never move, delete or re-point a
tag: the crates on crates.io are fixed, so bump the version instead and start
again at step 1.

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
   self-review" on (a solo maintainer: see the next subsection). Deployment branches and tags: "Selected branches and tags",
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

### Solo maintainer

With one maintainer the settings above cannot all hold literally. These exact
settings keep the protections that still mean something:

- **Environment `release`:** Required reviewers: yourself (or a second account
  you control). "Prevent self-review": **off**, otherwise no one can approve.
  The Environment still stops a run until you approve each of the two jobs, and
  limits it to `v*` tags.
- **Ruleset for `main`:** require the CI status checks (the jobs named in step 2
  of the order above), **0** required approvals, block force pushes and
  deletion. Changes reach `main` only through a pull request whose checks passed.
- **Tag ruleset (`v*`):** restrict creation, updates and deletion, and put
  yourself on the bypass list, so you can create the release tag and, before the
  first `cargo publish`, delete a failed one.

None of this is review by a second person. It guards against mistakes and
stolen tokens, not against the maintainer.

## Binary release (GitHub, never crates.io)

`.github/workflows/release.yml` builds the CLI for seven targets, writes
`SHA256SUMS` and a CycloneDX SBOM, attests build provenance, and creates a
**draft** GitHub release. It publishes nothing to crates.io, never publishes the
GitHub release itself, and holds no secret. It has **not been run**; its first
run is the real `v0.1.0` tag (step 6 above), so read `docs/release-pins.md`
first: the placeholder pins block the run until they are replaced.

1. Before tagging, fill in the pins (`docs/release-pins.md`). Then
   `./scripts/check-pins.sh --release` must pass and `cargo test` must pass with
   `LIBTEA_ALLOW_PIN_PLACEHOLDERS` unset. The workflow runs the same script first
   and fails the whole run if a placeholder remains.
2. After the tag is pushed, watch the run and approve the two deployments:

   ```sh
   VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' crates/tea-cli/Cargo.toml | head -n 1)
   gh run list --workflow release.yml --limit 3
   gh run watch                      # pick the run for the tag
   gh release view "v$VERSION"
   gh release download "v$VERSION" --dir /tmp/libtea-release
   ```

   Then follow `docs/installing.md` on the downloaded files (checksums,
   `gh attestation verify ... --source-ref "refs/tags/v$VERSION"`, run the binary
   on at least one platform you have, open the SBOM).
3. The release stays a draft until step 12 of the crate order. A tag with a
   suffix (for example `v0.1.1-rc.1`) produces a draft *prerelease* with file
   names carrying the whole tag version; that is kept for future use and is not
   part of the first release. Every tag, suffix or not, must be on `main`.

## How the CLI is distributed, and what folding conformance in would involve

`libtea-cli` and `libtea-conformance` stay `publish = false` for 0.1.0. The CLI
ships two ways: the release binaries from the workflow above, and
`cargo install --git https://github.com/Mehrn0ush/libtea-rs --locked libtea-cli`
(builds from source at the ref you give with `--tag`; no checksum or attestation
check is involved in that route). Neither goes through crates.io.

`libtea-cli` depends on `libtea-conformance` for its `conformance` command, which
is why the CLI cannot be published first: a published crate may not depend on an
unpublished one. Folding the conformance code into `libtea-cli` at 0.2 would
mean: moving its modules and the checked-in fixtures under the CLI crate, keeping
the JSON report shape (`docs/cli-json.md`) and the doctests that compile the
README snippets, removing the `libtea-conformance` package and updating the
workspace, the CI path filters and `docs/architecture.md`, and then deciding
whether the CLI name `libtea-cli` is acceptable on crates.io. This is a plan, not
done.
