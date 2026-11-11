# Release pins

The release workflow and the Action use third-party code. Every `uses:` of a
third-party action is pinned to a full commit SHA with a `# vX.Y.Z` comment, and
the SBOM tool has an exact version. Pinned on 2026-11-08:

| What | Where | Pin | Release |
| --- | --- | --- | --- |
| `actions/upload-artifact` | `release.yml` (3 uses) | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | v7.0.2 |
| `actions/download-artifact` | `release.yml` (2 uses) | `9000827ccba6bdab643e8b6fd33ac0654aef8333` | v8.0.2 |
| `actions/attest-build-provenance` | `release.yml` | `4d101475d8b20a2381f78447822ac1eab6504dd8` | v4.2.2 |
| `cargo-cyclonedx` | `release.yml`, `sbom` job `env:` | `0.5.10` (`cargo install --locked`) | 0.5.10 |
| `actions/cache` | `action.yml` | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` | v6.1.0 |
| `actions/checkout` | every workflow (12 uses) | `3d3c42e5aac5ba805825da76410c181273ba90b1` | v7.0.1 |
| `actions/upload-artifact` | `interop-manual.yml`, `spec-drift.yml` (as in `release.yml`) | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` | v7.0.2 |
| runner images | `release.yml` | `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `windows-2022` | |

On 2026-11-09 `actions/checkout` and `actions/upload-artifact` were moved to
releases that run on Node 24 (`runs.using: node24`; the previous checkout pin
produced a "Node.js 20 is deprecated" notice on every CI job). Each SHA was
resolved from the tag two ways (`commits/<tag>` and `git/ref/tags/<tag>`) and
the two agreed.

Each SHA was resolved from the tag two ways (`gh api repos/<action>/commits/<tag>`
and the tag ref `git/ref/tags/<tag>`, dereferenced if annotated) and the two
agreed. `upload-artifact` v7 and `download-artifact` v8 are the current releases
of each; they are not the same major version. `cargo install cargo-cyclonedx
--version 0.5.10 --locked` was run in a scratch directory and `scripts/make-sbom.sh`
was run with it (157 components for the CLI). Since then the release workflow
has run with these pins for every tag from `v0.1.0` to `v0.1.5`.

The `taiki-e/install-action` uses in `ci.yml` (`cargo-audit`,
`cargo-semver-checks`) are tag-pinned, not SHA-pinned. `unpinned_third_party_actions_are_exactly_the_known_debt`
in `crates/tea-cli/tests/workflow_policy.rs` lists them.

## The gate

- `scripts/check-pins.sh --release` is the first step of the release workflow,
  after checkout. A tag cannot produce a release while any `PIN_ME_*`
  placeholder is present. It never honors the development flag below.
- `cargo test` (`unresolved_pins_block_unless_the_documented_flag_is_set` in
  `crates/tea-cli/tests/pin_gate.rs`) fails while a placeholder is present,
  unless `LIBTEA_ALLOW_PIN_PLACEHOLDERS=1` is set. The flag is for development
  only and is no longer set anywhere in `ci.yml`; a test fails if it comes back.
- A placeholder in a `uses:` would also make GitHub fail the job at setup.

## Refreshing a pin

To move a pin, or to redo this from placeholders, the commands below apply.
`the_documented_commands_resolve_every_placeholder` runs the `pins-apply` block
in a scratch copy of the repository (turned back into placeholders first) with
made-up values.

Run from the repository root with the `gh` CLI logged in. Take the SHA from the
repository named, never from a blog post or another workflow, and read each
release's notes before choosing a tag.

```sh
# 1. The latest release tag of each action (choose a tag you trust).
for action in upload-artifact download-artifact attest-build-provenance cache; do
  printf '%s  ' "$action"
  gh api "repos/actions/$action/releases/latest" --jq .tag_name
done

# 2. Resolve the tag you chose to a commit SHA (works for annotated and
#    lightweight tags). Replace each <TAG> with the tag from step 1.
UPLOAD_SHA=$(gh api repos/actions/upload-artifact/commits/<TAG> --jq .sha)
DOWNLOAD_SHA=$(gh api repos/actions/download-artifact/commits/<TAG> --jq .sha)
ATTEST_SHA=$(gh api repos/actions/attest-build-provenance/commits/<TAG> --jq .sha)
CACHE_SHA=$(gh api repos/actions/cache/commits/<TAG> --jq .sha)
printf '%s\n' "$UPLOAD_SHA" "$DOWNLOAD_SHA" "$ATTEST_SHA" "$CACHE_SHA"   # four 40-hex lines

# 3. The cargo-cyclonedx version: the first line reads  cargo-cyclonedx = "X.Y.Z".
cargo search cargo-cyclonedx --limit 1
CYCLONEDX=X.Y.Z    # the number from that line
```

Then apply them (this block is tested: `the_documented_commands_resolve_every_placeholder`
runs it in a scratch copy with made-up values):

```sh pins-apply
perl -pi -e "s/PIN_ME_UPLOAD_ARTIFACT/$UPLOAD_SHA/g" .github/workflows/release.yml
perl -pi -e "s/PIN_ME_DOWNLOAD_ARTIFACT/$DOWNLOAD_SHA/g" .github/workflows/release.yml
perl -pi -e "s/PIN_ME_ATTEST_BUILD_PROVENANCE/$ATTEST_SHA/g" .github/workflows/release.yml
perl -pi -e "s/PIN_ME_CARGO_CYCLONEDX_VERSION/$CYCLONEDX/g" .github/workflows/release.yml
perl -pi -e "s/PIN_ME_ACTIONS_CACHE/$CACHE_SHA/g" action.yml
# Retire the development flag: every line that mentions it (the comment and the setting).
perl -ni -e 'print unless /LIBTEA_ALLOW_PIN_PLACEHOLDERS/' .github/workflows/ci.yml
./scripts/check-pins.sh --release
```

After that:

```sh
git diff --stat                 # release.yml, action.yml, ci.yml only
cargo test -p libtea-cli --test pin_gate --test workflow_policy --test action_policy
```

with the flag **unset**. Commit the three files together.

## Also before the first tag

1. `scripts/make-sbom.sh` was run with `cargo-cyclonedx` 0.5.10. That run showed
   that the tool writes one SBOM per workspace member, which the script did not
   expect; it now keeps the CLI's SBOM and removes the rest
   (`make_sbom_keeps_the_cli_sbom_when_the_tool_writes_one_per_member`). The
   workflow's `sbom` job has still not run on GitHub.
2. Create the `release` Environment and the tag ruleset described in
   `docs/release-checklist.md` ("Repository settings required before the first
   tag"). The workflow runs both privileged jobs in that Environment; without a
   required reviewer it stops nothing, and without the tag ruleset anyone who can
   push a tag can start a run.
3. Follow the release order in `docs/release-checklist.md`; the workflow's first run is the real tag.

The tag-pinned actions in `ci.yml` and the other workflows are listed above.
