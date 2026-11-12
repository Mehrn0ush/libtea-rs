# Release checklist (libtea-core → libtea-client)

Do **not** publish `libtea-cli` or `libtea-conformance` (`publish = false`).

A green **local** `cargo doc` is **not** sufficient. CI must be green on the
feature branch and again on `main` before anything is tagged or published.

## Exact order

1. **Record the reviewed SHA**: `git rev-parse HEAD` on the feature branch tip
   that was reviewed.
2. **CI green on the feature branch** at that SHA: `test` (stable and MSRV
   1.89.0, including the `cargo doc` step with `RUSTDOCFLAGS=-D warnings`),
   `package`, `cargo deny` and `cargo audit` (both gate on RustSec advisories),
   and the Windows and macOS test jobs. `cargo semver-checks` skips each crate only while crates.io returns
   404 for it (first release). It fails on any other lookup error. Since 0.1.0
   was published it runs against the latest published version and is required.
3. **Merge to `main` without squashing** (merge commit or fast-forward), so
   the reviewed commits land unchanged (check that each builds with
   `git rebase -x 'cargo check --workspace --all-targets --locked' <base>` on a
   throwaway branch).
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
   on until the checks succeed. Nothing has been published to crates.io at this point.
8. Publish `libtea-core` (see "Publishing with a token" below):
   `cargo publish -p libtea-core --locked`.
9. **Wait** until the index serves `libtea-core 0.1.0`, for example until
   `cargo info libtea-core@0.1.0` (or `cargo search libtea-core`) shows it.
10. `cargo publish --dry-run -p libtea-client --locked` (this resolves
    `libtea-core` from crates.io).
11. `cargo publish -p libtea-client --locked`.
12. **Publish the GitHub draft: publish the release by hand** (the workflow
    never does).
13. **Post-publish checks:** the commands are in "Post-publish checks" below.

**If something fails.** Before the first `cargo publish` (steps 5 to 7: the
workflow, or the verification of the draft): the maintainer deletes the draft
release and the tag (`gh release delete v0.1.0 --yes --cleanup-tag`, which needs
the tag ruleset's bypass list), fixes the problem on a branch, merges it to
`main` by pull request, records the new `RELEASE_SHA`, and tags that commit.
After the first `cargo publish` succeeded, never move, delete or re-point a
tag: the crates on crates.io are fixed, so bump the version instead and start
again at step 1.

## Publishing with a token

Do not use `cargo login`: it echoes the token you paste. Create a short-lived
token on crates.io with only the scopes `publish-new` and `publish-update`,
then, in the terminal you publish from:

```sh
read -rs CARGO_REGISTRY_TOKEN
export CARGO_REGISTRY_TOKEN
cargo publish -p libtea-core --locked
unset CARGO_REGISTRY_TOKEN
```

`read -rs` shows nothing as you paste. Publish `libtea-client` the same way
after the index serves `libtea-core` (steps 9 to 11), then **revoke the token on
crates.io**. Never paste a token into a chat, a ticket or a terminal log that is
kept.

## Lessons from the first release

- **`gh` version.** `gh attestation verify --source-ref` needs a `gh` that has
  that flag (verified with 2.103.3). `docs/installing.md` has the
  `--cert-identity` form for older versions.
- **Actions permissions.** Settings, Actions, General must be "Allow all
  actions and reusable workflows". `local_only` makes every workflow fail with
  `startup_failure` before any job starts.
- **Redoing a release before the first `cargo publish`.**
  `gh release delete vX.Y.Z --yes --cleanup-tag` removes the draft and the remote
  tag (it needs the tag ruleset's bypass); `git tag -d vX.Y.Z` removes the local
  one. Then fix on a branch, merge by pull request and tag the new `main`
  commit.
- **A tag push also starts CI**, so the newest run on the tag is not
  necessarily the release run. Find it with
  `gh run list --workflow release.yml --limit 3`, then `gh run watch <id>`.
- **Shell blocks meant for pasting have no inline `# comments`.** An
  interactive zsh does not treat `#` as a comment unless `interactive_comments`
  is set, and the paste fails. Put the explanation above the block.

## Post-publish checks

Run these after step 12, from a directory outside the repository.

```sh
cargo info libtea-core@0.1.0
cargo info libtea-client@0.1.0
cargo new --bin smoke && cd smoke
cargo add libtea-client@0.1.0 libtea-core@0.1.0
cargo add tokio --features macros,rt-multi-thread
```

Replace `src/main.rs` with the library snippet from the client README, then:

```sh
cargo build
cargo install --git https://github.com/Mehrn0ush/libtea-rs --tag v0.1.0 --locked libtea-cli
libtea --version
gh release view v0.1.0 --repo Mehrn0ush/libtea-rs --json isDraft,assets
```

Verify one archive as `docs/installing.md` describes. Then open
https://docs.rs/libtea-core/0.1.0 and https://docs.rs/libtea-client/0.1.0 and
check that both build and show the independence, no-certification and
checksum-is-not-authenticity statements. The crates.io pages show the README
packaged with the release; README changes made later appear there only with the
next publish.

## Local preflight (does not replace CI)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo +1.89.0 test --workspace --all-targets --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo deny check
cargo audit
cargo package -p libtea-core --locked
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
   request (0 approvals when you are the only maintainer, see
   "Solo maintainer" below), require status checks (the CI jobs), block
   force pushes and deletion. The workflow refuses any tag whose commit is not on
   `main`; this makes "on main" mean "reviewed".
4. **Private vulnerability reporting** (Settings, Code security): enable it and
   make sure `SECURITY.md` says how to use it.
5. **Actions settings** (Settings, Actions, General): "Workflow permissions" set
   to read-only by default; "Allow GitHub Actions to create and approve pull
   requests" off. The release workflow declares its own permissions per job.
   Also set "Actions permissions" to **"Allow all actions and reusable
   workflows"**. "Allow <owner>, and select non-<owner>, actions" works only if
   every action the workflows use is on the list. "Disable Actions" and the
   "local only" choice (`allowed_actions: local_only`) break every workflow: runs
   fail with `startup_failure` before any job starts. `scripts/check-repo-settings.sh`
   reports `local_only` when the token may read the setting (admin rights); the
   scheduled workflow's `GITHUB_TOKEN` cannot, so it says "not verified".
6. **Immutable releases** (Settings, General, if your plan offers it): turn it on
   so that a published release's assets cannot be replaced later. The Action's
   `SHA256SUMS` check is only as strong as the release's assets are fixed.

### Creating the rulesets

Branch rules and rulesets are free on a **public** repository; a private
repository on a free account cannot use them. Make the repository public first.
The commands below create the two rulesets for the solo-maintainer settings
below. This repository's rulesets (`main` and `release tags`) were set up to
match them, and `scripts/check-repo-settings.sh` checks the result; on a new
repository, read the result back:

```sh
REPO=Mehrn0ush/libtea-rs

# main: pull request required (0 approvals), CI checks required, no force push
# or deletion, nobody bypasses. Merge commits stay allowed (no linear history).
gh api -X POST "repos/$REPO/rulesets" --input - <<'JSON'
{
  "name": "main",
  "target": "branch",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] } },
  "bypass_actors": [],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    { "type": "pull_request", "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": false } },
    { "type": "required_status_checks", "parameters": {
        "strict_required_status_checks_policy": true,
        "required_status_checks": [
          { "context": "rust stable" },
          { "context": "rust 1.89.0" },
          { "context": "package" },
          { "context": "cargo deny" },
          { "context": "cargo audit" },
          { "context": "cargo semver-checks" },
          { "context": "non_exhaustive guards" },
          { "context": "tests on windows-2022" },
          { "context": "tests on macos-15" } ] } }
  ]
}
JSON

# v* tags: create, update, delete and force push blocked; repository admins bypass.
gh api -X POST "repos/$REPO/rulesets" --input - <<'JSON'
{
  "name": "release tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["refs/tags/v*"], "exclude": [] } },
  "bypass_actors": [ { "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" } ],
  "rules": [
    { "type": "creation" }, { "type": "update" },
    { "type": "deletion" }, { "type": "non_fast_forward" }
  ]
}
JSON

# Merge commits only, as the pull request says.
gh api -X PATCH "repos/$REPO" -F allow_merge_commit=true -F allow_squash_merge=false -F allow_rebase_merge=false

./scripts/check-repo-settings.sh "$REPO" main
```

The required checks include `tests on windows-2022` and `tests on macos-15`
(the `test-os` job in `ci.yml`). To add them to an existing `main` ruleset, read
it, append the two contexts and write it back (the ruleset id is in
`gh api repos/$REPO/rulesets`):

```sh
gh api "repos/$REPO/rulesets/$ID" >ruleset.json
jq '{name,target,enforcement,conditions,bypass_actors,rules: (.rules | map(if .type=="required_status_checks" then .parameters.required_status_checks += [{"context":"tests on windows-2022"},{"context":"tests on macos-15"}] else . end))}' ruleset.json >ruleset-new.json
gh api -X PUT "repos/$REPO/rulesets/$ID" --input ruleset-new.json
```

`cargo audit` is required as well; it reads the same RustSec database as
`cargo deny` and is a second opinion on it. A new advisory against a
dependency then blocks every pull request until the dependency is updated or
the advisory is assessed; do not silence it in `deny.toml` or `audit.toml`
without a written reason. `.github/workflows/repo-settings.yml` runs the same check
on every push to `main` and weekly, and fails when a ruleset is missing or
weakened. It reports; it cannot block anything, and someone with admin rights can
change the rulesets or the workflow. The rulesets are the protection.

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
GitHub release itself, and holds no secret. It has run for every tag from `v0.1.0` to
`v0.1.6`, all successfully (`v0.1.6`: run 43182817134). Read `docs/release-pins.md` before changing it: it
lists the pins and how to refresh them.

1. Before tagging, check the pins (`docs/release-pins.md`):
   `./scripts/check-pins.sh --release` must pass and `cargo test` must pass with
   `LIBTEA_ALLOW_PIN_PLACEHOLDERS` unset. The workflow runs the same script first
   and fails the whole run if a placeholder is present.
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

`libtea-cli` and `libtea-conformance` stay `publish = false` in 0.1.x. The CLI
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
