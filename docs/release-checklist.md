# Release checklist (libtea-core → libtea-client → libtea-cli)

`libtea-doctests` is never published (`publish = false`). `libtea-cli` is
published from 0.2.0 (`docs/design/cli-publish.md`); 0.1.x of the CLI is on
GitHub Releases only.

A green **local** `cargo doc` is **not** sufficient. CI must be green on the
feature branch and again on `main` before anything is tagged or published.

## What a release contains, and its tag

The CLI and the libraries have separate versions (the CLI goes to 0.2.0 while
`libtea-core` and `libtea-client` stay on 0.1.x). A release is one of:

| Release | Tag | Release workflow | Published to crates.io |
| --- | --- | --- | --- |
| CLI only | `v<cli version>` | yes: binaries, SBOM, attestation, draft | `libtea-cli` |
| Libraries only | `libs-v<library version>` | no | the libraries that changed |
| Both | `v<cli version>` (and `libs-v<library version>` on the same commit) | yes | the libraries, then `libtea-cli` |

`v<version>` is checked against `crates/tea-cli/Cargo.toml` by
`scripts/check-release-tag.sh`; the binaries, the Action's download URL and the
attestation's `--source-ref` all use it. A `libs-v` tag does not match the
release workflow's `v[0-9]+...` trigger and starts nothing. `libs-v0.1.9`
(2026-11-20) was the first library-only release: `libtea-core` and
`libtea-client` 0.1.9, no release run, no GitHub release. `libs-v0.2.0` and
`v0.2.2` (2026-11-21) were the first "Both" release, on one commit.

Publish in dependency order: `libtea-core`, then `libtea-client`, then
`libtea-cli`, each only after the index serves the one before. Publish only
the crates whose version was bumped; a crate whose version is already on
crates.io is skipped (`cargo publish` refuses it anyway). A library left at its
version keeps its published version: the CLI's `Cargo.toml` names the library
versions it needs, so check that those are on crates.io before tagging.

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
5. From a clean checkout of `RELEASE_SHA`: `cargo publish --dry-run -p libtea-core --locked`
   if `libtea-core` is released. For a CLI release whose libraries are all
   already on crates.io, also `cargo package -p libtea-cli --locked` (the full
   verify, which builds against the libraries from crates.io); otherwise that
   check waits for step 12.
6. **Tag `RELEASE_SHA`** once, with the tag from the table above:
   `git tag -a v0.2.0 RELEASE_SHA -m "libtea 0.2.0"` for a CLI release, or
   `git tag -a libs-v0.1.9 RELEASE_SHA -m "libtea libraries 0.1.9"` for the
   libraries alone, and push it. The tag points at the `main` commit, never at a
   feature-branch commit. There is no separate trial tag.
7. **Binary release workflow** (CLI releases only; details in "Binary release"
   below). The `v` tag starts `release.yml`. Approve the two deployments (`attest`,
   `draft-release`), download the **draft** release, and verify it with
   `docs/installing.md`: checksums, `gh attestation verify` with
   `--source-ref refs/tags/v$VERSION`, run the binary, open the SBOM. Do not go
   on until the checks succeed. Nothing has been published to crates.io at this point.
8. Publish `libtea-core` if it is released (see "Publishing with a token" below):
   `cargo publish -p libtea-core --locked`.
9. **Wait** until the index serves it, for example until
   `cargo info libtea-core@0.1.9` (or `cargo search libtea-core`) shows it.
10. If `libtea-client` is released: `cargo publish --dry-run -p libtea-client --locked`
    (this resolves `libtea-core` from crates.io).
11. `cargo publish -p libtea-client --locked`, then wait until the index serves
    it, as in step 9.
12. If `libtea-cli` is released: `cargo package -p libtea-cli --locked` (the
    full verify; it resolves the libraries from crates.io, so it can only pass
    once they are there), then `cargo publish --dry-run -p libtea-cli --locked`.
13. `cargo publish -p libtea-cli --locked`.
14. **Publish the GitHub draft: publish the release by hand** (CLI releases
    only; the workflow never does).
15. **Post-publish checks:** the commands are in "Post-publish checks" below.
16. **Homebrew formula** (CLI releases only): regenerate `Formula/libtea.rb`
    from the verified `SHA256SUMS` and open a pull request to
    `Mehrn0ush/homebrew-tap` (see "Homebrew tap" below).

**If something fails.** Before the first `cargo publish` (steps 5 to 7: the
workflow, or the verification of the draft): the maintainer deletes the draft
release and the tag (`gh release delete v0.2.0 --yes --cleanup-tag`, which needs
the tag ruleset's bypass list; for a `libs-v` tag, which has no release,
`git push origin :refs/tags/libs-v0.1.9`), fixes the problem on a branch,
merges it to `main` by pull request, records the new `RELEASE_SHA`, and tags
that commit. After the first `cargo publish` succeeded, never move, delete or
re-point a tag: the crates on crates.io are fixed, so bump the version of the
crate that needs the fix instead and start again at step 1.

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

`read -rs` shows nothing as you paste. Publish `libtea-client` and
`libtea-cli` the same way, each after the index serves the crate before it
(steps 9 to 13), then **revoke the token on crates.io**. The first publish of
`libtea-cli` creates a new crate, which needs the `publish-new` scope. Never paste a token into a chat, a ticket or a terminal log that is
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
  That CI run on the tag is not a release gate (CI on `main` at
  `RELEASE_SHA` is). For `v0.2.1` its macOS job was cancelled because no
  hosted runner picked it up; `gh run rerun <id> --failed` re-ran it green.
- **docs.rs lags crates.io.** A new version shows "The requested version does
  not exist" on docs.rs until its build leaves the queue
  (https://docs.rs/releases/queue). That is not a failed publish; check
  `cargo info`, and look at `https://docs.rs/crate/<crate>/<version>/builds`
  only if the page is still missing hours later.
- **`scripts/check-repo-settings.sh` takes the repository**:
  `./scripts/check-repo-settings.sh Mehrn0ush/libtea-rs` (in a workflow it
  defaults to `$GITHUB_REPOSITORY`).
- **Shell blocks meant for pasting have no inline `# comments`.** An
  interactive zsh does not treat `#` as a comment unless `interactive_comments`
  is set, and the paste fails. Put the explanation above the block.

## Post-publish checks

Run these after step 14, from a directory outside the repository, for the
crates and versions you released.

Libraries:

```sh
cargo info libtea-core@0.1.9
cargo info libtea-client@0.1.9
cargo new --bin smoke && cd smoke
cargo add libtea-client@0.1.9 libtea-core@0.1.9
cargo add tokio --features macros,rt-multi-thread
```

Replace `src/main.rs` with the library snippet from the client README, then
`cargo build`.

CLI:

```sh
cargo info libtea-cli@0.2.0
cargo install libtea-cli --version 0.2.0 --locked
libtea --version
gh release view v0.2.0 --repo Mehrn0ush/libtea-rs --json isDraft,assets
```

Verify one archive as `docs/installing.md` describes. For a library release,
open https://docs.rs/libtea-core/0.1.9 and https://docs.rs/libtea-client/0.1.9
and check that both build and show the independence, no-certification and
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
cargo package -p libtea-cli --locked --no-verify --list
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
   `v*` and `libs-v*`. Enforcement: Active. Rules: restrict creations, restrict updates,
   restrict deletions, block force pushes. Bypass list: only the maintainers who
   may release. Without it anyone with write access can create a `v*` tag.
   A `libs-v*` tag starts no workflow, but it names what was published, so it
   gets the same protection.
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

# v* and libs-v* tags: create, update, delete and force push blocked; repository admins bypass.
gh api -X POST "repos/$REPO/rulesets" --input - <<'JSON'
{
  "name": "release tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["refs/tags/v*", "refs/tags/libs-v*"], "exclude": [] } },
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

The `libs-v*` pattern was added for library-only releases (issue #14). To add
it to an existing `release tags` ruleset before the first such tag:

```sh
gh api "repos/$REPO/rulesets/$ID" >tag-ruleset.json
jq '{name,target,enforcement,conditions: (.conditions | .ref_name.include += ["refs/tags/libs-v*"]),bypass_actors,rules}' tag-ruleset.json >tag-ruleset-new.json
gh api -X PUT "repos/$REPO/rulesets/$ID" --input tag-ruleset-new.json
```

`scripts/check-repo-settings.sh` requires both patterns, `v*` and `libs-v*`. This
repository's ruleset has had both since 2026-11-20.

`cargo audit` is required as well; it reads the same RustSec database as
`cargo deny` and is a second opinion on it. A new advisory against a
dependency then blocks every pull request until the dependency is updated or
the advisory is assessed; do not silence it in `deny.toml` or `audit.toml`
without a written reason. When an advisory affects a dependency in the
`Cargo.lock` of a published release, record the assessment in
`vex/libtea.vex.cdx.json` (SECURITY.md): add a statement for the supported
releases, bump `version` and `metadata.timestamp`, and keep `analysis.detail`
to what was checked. `crates/tea-cli/tests/vex.rs` checks its shape. `.github/workflows/repo-settings.yml` runs the same check
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
- **Tag ruleset (`v*`, `libs-v*`):** restrict creation, updates and deletion, and put
  yourself on the bypass list, so you can create the release tag and, before the
  first `cargo publish`, delete a failed one.

None of this is review by a second person. It guards against mistakes and
stolen tokens, not against the maintainer.

## Binary release (GitHub, never crates.io)

`.github/workflows/release.yml` builds the CLI for seven targets, writes
`SHA256SUMS` and a CycloneDX SBOM, attests build provenance, and creates a
**draft** GitHub release. It publishes nothing to crates.io, never publishes the
GitHub release itself, and holds no secret. It has run for every tag from `v0.1.0` to
`v0.2.2`, all successfully (`v0.1.6`: run 43182817134; `v0.1.7`: run 44500983192; `v0.1.8`: run 44584585018; `v0.2.0`: run 44691175896; `v0.2.1`: run 44783205174; `v0.2.2`: run 44895512012). Read `docs/release-pins.md` before changing it: it
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
3. The release stays a draft until step 14 of the order. A tag with a
   suffix (for example `v0.1.1-rc.1`) produces a draft *prerelease* with file
   names carrying the whole tag version; that is kept for future use and is not
   part of the first release. Every tag, suffix or not, must be on `main`.

## How the CLI is distributed

The CLI ships three ways:

- the release archives from the workflow above, with `SHA256SUMS` and a
  build-provenance attestation;
- from 0.2.0, `cargo install libtea-cli --locked` from crates.io (published by
  hand in steps 12 and 13; no checksum file or attestation);
- `cargo install --git https://github.com/Mehrn0ush/libtea-rs --tag vX.Y.Z --locked libtea-cli`,
  which builds the tagged source (no checksum or attestation either);
- from 0.2.2, `brew install mehrn0ush/tap/libtea` from the
  `Mehrn0ush/homebrew-tap` repository. It installs the release archives, and
  Homebrew checks the SHA-256 in the formula.

### Homebrew tap

`Formula/libtea.rb` in `Mehrn0ush/homebrew-tap` is generated, never edited
by hand. After step 14, in a new empty directory:

```sh
gh release download vX.Y.Z --repo Mehrn0ush/libtea-rs --pattern SHA256SUMS
gh attestation verify SHA256SUMS --repo Mehrn0ush/libtea-rs --signer-workflow Mehrn0ush/libtea-rs/.github/workflows/release.yml --source-ref refs/tags/vX.Y.Z
python3 -I /path/to/libtea-rs/scripts/make-homebrew-formula.py --version X.Y.Z --sums SHA256SUMS >libtea.rb
```

Copy `libtea.rb` to `Formula/libtea.rb` on a branch of the tap, open a pull
request, and after the merge check with `brew update`,
`brew upgrade mehrn0ush/tap/libtea` (or `brew install`), `brew test libtea`,
and `libtea --version`. macOS gets the darwin archives, and Linux gets the
statically linked musl archives. The `x86_64-apple-darwin` archive is
cross-compiled and not run by the release smoke test, so `brew test` on an
Intel Mac is its first run. Nothing updates the tap automatically: that
would need a token that can push to it.

`docs/installing.md` describes each. Until 0.1.8 the CLI depended on the
unpublished `libtea-conformance` crate, which kept it off crates.io; that code
now lives in `crates/tea-cli/src/conformance.rs` (`docs/design/cli-publish.md`,
issue #14).
