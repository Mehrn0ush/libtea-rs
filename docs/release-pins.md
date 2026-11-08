# Pins the release workflow still needs

`.github/workflows/release.yml` was written without network access, so four
values that must come from the internet are placeholders. A placeholder makes
the workflow fail at job setup (an action reference that does not exist, or an
unpinned `cargo-cyclonedx` version), so it cannot run by accident with a wrong
value. Fill them in before the first tag, dry run included.

| Placeholder in `release.yml` | What to fetch | Where it goes |
| --- | --- | --- |
| `actions/upload-artifact@PIN_ME_UPLOAD_ARTIFACT` (3 uses) | The commit SHA of a released v4-or-later tag of `actions/upload-artifact` | Replace `PIN_ME_UPLOAD_ARTIFACT` with the 40-character SHA, in all three places |
| `actions/download-artifact@PIN_ME_DOWNLOAD_ARTIFACT` (2 uses) | The commit SHA of a released tag of `actions/download-artifact` whose major version matches the upload action's (both v4 or both v5, and so on) | Replace `PIN_ME_DOWNLOAD_ARTIFACT` |
| `actions/attest-build-provenance@PIN_ME_ATTEST_BUILD_PROVENANCE` | The commit SHA of a released tag of `actions/attest-build-provenance` | Replace `PIN_ME_ATTEST_BUILD_PROVENANCE` |
| `actions/cache@PIN_ME_ACTIONS_CACHE` in `action.yml` (the GitHub Action, `docs/github-action.md`) | The commit SHA of a released tag of `actions/cache` | Replace `PIN_ME_ACTIONS_CACHE` in `action.yml` (one place) |
| `CARGO_CYCLONEDX_VERSION: PIN_ME_CARGO_CYCLONEDX_VERSION` | The exact version of the `cargo-cyclonedx` crate to install, as `x.y.z` | Replace the value in the `sbom` job's `env:` |

## How to get a SHA

With the GitHub CLI (this resolves annotated and lightweight tags alike):

```sh
gh api repos/actions/upload-artifact/commits/<TAG> --jq .sha
gh api repos/actions/download-artifact/commits/<TAG> --jq .sha
gh api repos/actions/attest-build-provenance/commits/<TAG> --jq .sha
gh api repos/actions/cache/commits/<TAG> --jq .sha
```

Pick `<TAG>` from the project's releases page, read its release notes, and
keep the tag as a trailing comment on the `uses:` line, for example
`uses: actions/upload-artifact@<sha> # v4.6.2`. Do not take a SHA from a blog
post or an unrelated workflow; take it from the repository named above.

For the crate version: `cargo search cargo-cyclonedx --limit 1`, or the
crate's page on crates.io. The workflow runs `cargo install --locked --version`,
so the exact version and the crate's own `Cargo.lock` decide what gets built.

## After filling them in

1. Remove the three `release.yml` entries from `KNOWN_UNPINNED` in
   `crates/tea-cli/tests/workflow_policy.rs`
   (`unpinned_third_party_actions_are_exactly_the_known_debt` fails until you do).
2. Run the gates that check nothing is left:
   `cargo test -p libtea-cli --test workflow_policy --test action_policy -- --ignored`.
3. Confirm `scripts/make-sbom.sh` matches the installed tool's flags
   (`cargo cyclonedx --help` should list `--manifest-path`, `--format` and
   `--spec-version`). The script was written from the tool's documentation, not
   run: the first dry run is where it is first exercised.
4. Create the `release` Environment and the tag ruleset described in
   `docs/release-checklist.md` ("Repository settings required before the first
   tag"). The workflow runs both of its privileged jobs in that Environment;
   without a required reviewer it would not stop anything, and without the tag
   ruleset anyone who can push a tag can start a run.
5. Run the dry run in `docs/release-checklist.md`.

The `action.yml` pin is checked by `action_has_no_unresolved_pins` (it is not
under `.github/workflows/`, so the debt list does not see it).

The four actions already used elsewhere in `ci.yml` and the other workflows
(`taiki-e/install-action` twice, `actions/upload-artifact@v4` twice) are still
tag-pinned; that older debt is tracked by the same test and is not part of this
list.
