# Release checklist (libtea-core → libtea-client)

Do **not** publish `libtea-cli` or `libtea-conformance` (`publish = false`).

A green **local** `cargo doc` is **not** sufficient. CI must be green on the
feature branch and again on `main` before anything is tagged or published.

## Exact order

1. **Record the reviewed SHA**: `git rev-parse HEAD` on the feature branch tip
   that was reviewed.
2. **CI green on the feature branch** at that SHA: `test` (stable and MSRV
   1.85.0, including the `cargo doc` step with `RUSTDOCFLAGS=-D warnings`),
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
cargo +1.85.0 test --workspace --all-targets --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo deny check
cargo audit
cargo package -p libtea-core --locked          # needs a clean tree
cargo package -p libtea-client --locked --no-verify --list
```

## Claims

State only what tests prove. Do not claim official conformance, universal
compatibility, or publisher-authenticity verification.
