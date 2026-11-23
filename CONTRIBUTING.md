# Contributing

Thank you for helping. This is a small project with a strict gate; the rules
below keep it that way.

## Before you open a pull request

Run the gate from the repository root. All of it must pass:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo test --workspace --doc --all-features --locked
cargo +1.89.0 test --workspace --all-targets --all-features --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
./scripts/check-non-exhaustive-guards.sh
cargo deny check
```

If you changed a workflow or a shell script, also run `actionlint` and
`shellcheck scripts/*.sh scripts/action/*.sh`. CI also runs `cargo audit`, which
gates; run it locally (`cargo install cargo-audit --locked`) when you change
dependencies.

CI also measures coverage and fuzzes; both gate. To run them locally:

```sh
cargo llvm-cov --workspace --all-features --locked --summary-only --fail-under-lines 90
(cd fuzz && cargo +nightly fuzz run <target> -- -max_total_time=30)
```

`cargo install cargo-llvm-cov --locked` and `cargo install cargo-fuzz --locked`
install the tools; `cargo +nightly fuzz list` names the targets. New
functionality comes with tests that keep line coverage at 90% or more. A crash
found by fuzzing is fixed with the crashing input added to that target's
corpus under `fuzz/corpus/`.

Do not lower a check, skip or ignore a test, or loosen a policy test to get a
green result. If a test is wrong, say so in the pull request and fix the test
for the right reason.

## Rules

- **Sign off every commit** with `git commit -s` (Developer Certificate of
  Origin). The sign-off line must match the author.
- **Pin actions by commit SHA.** Every `uses:` of a third-party action in a
  workflow or in `action.yml` is a full 40-character SHA followed by a
  `# vX.Y.Z` comment. Tools installed in CI have an exact version. The policy
  tests fail otherwise. See `docs/release-pins.md`.
- **One concern per pull request.** A bug fix, a refactor and a doc rewrite are
  three pull requests.
- **Tests with the change.** A fix comes with a test that fails without it. Docs
  that state a behavior are checked by tests where they can be (`SKILLS.md`,
  exit codes, workflow policy).
- **Merge commits only.** Pull requests are merged with a merge commit, not
  squashed or rebased, so reviewed commits land unchanged.
- **Public API.** `libtea-core` and `libtea-client` follow SemVer from 0.1.0
  (with the experimental items listed in the changelog). Add `#[non_exhaustive]`
  guards for new public types; `scripts/check-non-exhaustive-guards.sh` checks
  them.
- **Changelog.** User-visible changes get a line in `CHANGELOG.md`.

## Vulnerabilities

Do not open an issue or a pull request for a vulnerability. Report it privately
as described in [SECURITY.md](SECURITY.md).

## Bugs and interoperability reports

Use the issue templates. Never include credentials, API keys or tokens in an
issue, a log or a pull request.
