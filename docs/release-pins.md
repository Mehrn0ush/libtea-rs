# Pins the release workflow still needs

`.github/workflows/release.yml` and `action.yml` were written without network
access, so five values that must come from the internet are placeholders. They
fail closed:

- `scripts/check-pins.sh --release` is the first step of the release workflow,
  after checkout. A tag cannot produce a release while any `PIN_ME_*` remains.
  It never honors the development flag below.
- `cargo test` fails (`unresolved_pins_block_unless_the_documented_flag_is_set`
  in `crates/tea-cli/tests/pin_gate.rs`) while a placeholder remains, **unless**
  `LIBTEA_ALLOW_PIN_PLACEHOLDERS=1` is set in the environment. That flag is for
  development only: CI sets it in the test job while placeholders exist, and a
  test fails if the flag stays in `ci.yml` after the placeholders are gone.
  Locally, `export LIBTEA_ALLOW_PIN_PLACEHOLDERS=1` to run the whole suite
  before the pins are filled.
- A placeholder in a `uses:` also makes GitHub fail the job at setup, since no
  such ref exists.

| Placeholder | In | What it stands for |
| --- | --- | --- |
| `PIN_ME_UPLOAD_ARTIFACT` (3 uses) | `release.yml` | commit SHA of a released tag of `actions/upload-artifact` |
| `PIN_ME_DOWNLOAD_ARTIFACT` (2 uses) | `release.yml` | commit SHA of a released tag of `actions/download-artifact` (same major version as upload) |
| `PIN_ME_ATTEST_BUILD_PROVENANCE` | `release.yml` | commit SHA of a released tag of `actions/attest-build-provenance` |
| `PIN_ME_CARGO_CYCLONEDX_VERSION` | `release.yml`, `sbom` job `env:` | exact version `x.y.z` of the `cargo-cyclonedx` crate |
| `PIN_ME_ACTIONS_CACHE` | `action.yml` | commit SHA of a released tag of `actions/cache` |

## Fill them in

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

1. `scripts/make-sbom.sh` matches the installed `cargo-cyclonedx`: check
   `cargo cyclonedx --help` lists `--manifest-path`, `--format` and
   `--spec-version`. The script was written from the tool's documentation, not
   run; the first dry run is where it is first exercised.
2. Create the `release` Environment and the tag ruleset described in
   `docs/release-checklist.md` ("Repository settings required before the first
   tag"). The workflow runs both privileged jobs in that Environment; without a
   required reviewer it stops nothing, and without the tag ruleset anyone who can
   push a tag can start a run.
3. Run the dry run in `docs/release-checklist.md`.

The tag-pinned actions in `ci.yml` and the other workflows
(`taiki-e/install-action` twice, `actions/upload-artifact@v4` twice) are older
debt, tracked by `unpinned_third_party_actions_are_exactly_the_known_debt`; they
are not placeholders and are not part of this gate.
