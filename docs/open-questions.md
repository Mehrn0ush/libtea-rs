# Open questions

Questions that need an owner decision before implementation. Work continues
elsewhere without blocking on these.

1. **Signature formats for 0.2:** Which media types are in scope for the first
   real `SignatureVerifier` implementation (raw-over-digest only vs CMS/PGP)?
2. **Sigstore:** Separate Cargo feature vs separate crate?
3. **Trust anchors:** PEM files only, or also keyring / Sigstore TUF in 0.2?
4. **Digest binding:** Bind signatures to TEA checksums, HTTP `Repr-Digest`, or
   both?
5. **CLI publish:** Keep `publish = false`, or publish `libtea-cli` after a
   binary-name / install story review?
6. **Default TLS backend:** Stay rustls-only, or offer an optional `native-tls`
   feature later?
7. **Interop CI credentials:** Should the optional `workflow_dispatch` interop
   job use repository secrets, or remain documentation-only?

## Resolved 2026-11-07 (decisions from the maintainer; see `AUDIT.md` AUD-07, AUD-11, AUD-12, AUD-13, AUD-16)

8. **Proxy environment variables — RESOLVED.** Environment proxies stay
   honored by default. New `ClientBuilder::ignore_env_proxies(bool)` connects
   directly. `docs/security.md`, `README.md`, and the `libtea-client` crate
   rustdoc state that DNS-based destination checks are not enforced through a
   proxy. Test: `env_proxy_is_used_by_default_and_skipped_with_ignore_env_proxies`.

9. **`#[non_exhaustive]` — RESOLVED.** Added to the public enums and the
   config/outcome structs the library may extend; each struct has `Default`,
   a constructor, or `with_*` setters. The wire model structs (`Product`, `Page`,
   `Artifact`, …) were first left exhaustive and were made
   `#[non_exhaustive]` on 2026-11-08 (maintainer default); see the list in
   `CHANGELOG.md`. Tests: `compile_fail` doctest guards in each crate's
   `src/non_exhaustive_guards.rs` (`cargo test --doc`; checked by
   `scripts/check-non-exhaustive-guards.sh`), `crates/tea-client/tests/builders.rs`.

10. **SHA-1 — RESOLVED.** A match on weak algorithms only is
    `IntegrityOutcome::Weak`, not `Verified`. Opt-in:
    `ChecksumPolicy::with_allow_weak(true)`,
    `ClientBuilder::allow_weak_checksums(true)`, CLI `--allow-weak-checksums`.
    The CLI exits with status 4 on a weak-only match. Tests:
    `crates/tea-core/tests/weak_checksum*.rs`,
    `crates/tea-client/tests/weak_checksums.rs`,
    `weak_only_checksums_exit_4_unless_allowed`,
    `probe_reports_weak_only_checksums_by_exit_status`.

11. **Interop workflow secrets — RESOLVED.** The base URL (and TEI) come from
    the protected `tea-interop` Environment; no URL or TEI dispatch input;
    `workflow_dispatch` only; `permissions: {}` with `contents: read` on the
    job. Required Environment settings: `docs/interoperability.md`. Test:
    `crates/tea-cli/tests/workflow_policy.rs`. Configuring the Environment in
    GitHub is a maintainer step (`HANDOFF-COMMANDS.md`).

12. **CLI `--output` — RESOLVED.** Create-new by default; `--force` replaces a
    regular file through a temporary file and `rename`; a symbolic link as the
    final path component is refused with or without `--force` (parent
    directories are not checked and may be symlinks, by decision); no file name is derived from server
    data (none was before either; `Content-Disposition` is ignored). Tests:
    `crates/tea-cli/tests/output_file.rs`.

Still open: questions 1–6 above. Question 7 is answered by 11 (Environment-scoped secrets, if any).

## Decision for the maintainer: exact dependency pins (investigated 2026-11-08)

13. **`url`, `idna`, `idna_adapter`, `uuid` pins in the published manifests. RESOLVED 2026-11-08 (maintainer chose option 1, caret ranges).** Root `Cargo.toml` pins `url = "=2.5.4"`, `idna = "=1.0.3"`,
    `idna_adapter = "=1.2.0"` and `uuid = ">=1.26.0, <1.27.0"`; `libtea-core`
    uses them through `workspace = true`, so they ship in its manifest.
    `libtea-core` now also re-exports `url`, `semver`, and `uuid`, which makes
    those versions part of its API. An exact `=` requirement on `url` makes a
    downstream crate that needs any other `url` (for example one that requires
    `url >= 2.5.5`) fail to resolve.

    **Experiment (scratch copy only; the real `Cargo.toml` and `Cargo.lock` are
    unchanged).** Replace the pins with caret ranges (`url = "2"`,
    `idna = "1"`, `idna_adapter = "1"`, `uuid = "1"`), set the workspace
    `resolver = "3"` (MSRV-aware), delete `Cargo.lock`, `cargo generate-lockfile`:

    | | Current lock | Experiment lock |
    | --- | --- | --- |
    | `url` | 2.5.4 | 2.5.8 |
    | `idna` | 1.0.3 | 1.1.0 |
    | `idna_adapter` | 1.2.0 | 1.2.1 |
    | `icu_properties` / `icu_normalizer` | 1.5.x | 2.1.x |
    | `uuid` | 1.26.1 | 1.26.1 (1.27+ is skipped: it needs Rust 1.89) |
    | `wiremock` | 0.6.4 | 0.6.4 (0.6.5 skipped: needs 1.88; the dev pin stayed) |
    | crates whose locked version changed | | 44 of 218 |

    `cargo +1.85.0 test --workspace --locked` on the experiment lock: **passed**
    (367 passed, 0 failed, 11 ignored; unit, integration, and doctests).
    So with a lock file and the MSRV-aware resolver, 1.85 still builds without
    any pin.

    **The catch is downstream.** `resolver = "3"` and the committed lock apply
    to this workspace only. A consumer resolves for itself. Resolving the same
    ranges *without* MSRV awareness (`resolver = "2"`,
    `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow`) picks `uuid` 1.29.1
    (needs Rust **1.89**), `idna_adapter` 1.2.2 (**1.86**), and `icu_*` 2.4.0
    (**1.88**); the dev-dependency `proptest` 1.12.0 also needs 1.88. Capping
    only `idna_adapter < 1.2.2` is not enough: `idna_adapter` 1.2.1 still pulls
    `icu_*` 2.4.0 (checked: seven `icu_*` crates above 1.85 remain in the
    normal-dependency closure of `libtea-core` / `libtea-client`). A Rust 1.85
    consumer on edition 2021 (`resolver = "2"`, which does not fall back by
    default) would therefore get a build error from a dependency, not from
    this crate. That is what the exact pins prevent today.

    **Options.**

    1. **Loosen the pins to caret ranges.** Best for resolution and for the new
       re-exports; 1.85 users must use the MSRV-aware resolver
       (`resolver = "3"`, edition 2024, or
       `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`) or a lock file. Say
       so in the README.
    2. **Bump the MSRV.** Rust **1.88** removes every normal-dependency problem
       listed above except `uuid` (keep `uuid <1.27`, or go to **1.89**, which
       also allows `uuid` 1.29.1 and removes the need for any pin). Dev pins
       (`wiremock =0.6.4`, `proptest`) could then also be relaxed. Cost: users
       on 1.85 to 1.87 are excluded; the MSRV is a policy decision
       (`docs/milestones/0.7.md`, item 0.7-6).
    3. **Keep the pins and document them.** Zero change; the `=` pins on `url`
       and `idna` remain a resolution hazard for downstream crates, and the
       re-exports tie users to `url` 2.5.4 exactly.
    4. **Middle path (not tested).** Relax `url` to `>=2.5.4, <3` and `idna` to
       `>=1.0.3, <2`, keep `idna_adapter = ">=1.2.0, <1.2.1"` and the `uuid`
       cap as documented MSRV guards (these are ranges, not exact pins). I did
       not verify that `idna 1.1.0` accepts `idna_adapter 1.2.0`; the
       experiment above used `idna_adapter` 1.2.1.

    **Recommendation.** Option 4 if 1.85 must stay the MSRV for 0.1.0 (test it
    with `cargo +1.85.0 test --workspace --locked` and by building a consumer
    crate with `resolver = "2"`), and decide Option 2 for 0.2 together with the
    MSRV policy. Avoid Option 3: the `url` pin is the part most likely to hurt
    users. Do not change `Cargo.toml` before the first publish without
    re-running the full verification list.

14. **Server strings in library error Display. RESOLVED 2026-11-08 (option a: escape when the error is built).** The CLI now escapes
    control characters in everything it prints, but `ClientError`'s Display and
    Debug still carry raw server strings (`Token { error }`,
    `Unresolved { error, message }`, `Api { tea_error }`). A program that prints
    them to a terminal is exposed to escape-sequence injection.
    Options: (a) escape control characters when the error is constructed (small,
    changes the stored text, no API change); (b) escape in `Display` only;
    (c) document that callers must escape. **Recommendation:** (a) for 0.2; it
    is local and testable. Not done in this pass because the request limited
    F6 to what the CLI prints.

### Resolution of items 13 and 14 (2026-11-08)

**13 — applied.** Root `Cargo.toml`: `url = "2"`, `idna = "1"`, `idna_adapter = "1"`,
`uuid = { version = "1", ... }`, workspace `resolver = "3"`; `Cargo.lock`
regenerated (`url` 2.5.8, `idna` 1.1.0, `idna_adapter` 1.2.1, `icu_*` 2.1.x,
`uuid` 1.26.1, 44 of 215 crates changed). `cargo +1.85.0 test --workspace
--all-targets --locked`: 230 passed, 0 failed, 10 ignored; `cargo deny check`
ok. The READMEs say that 1.85 is verified with the committed lock file and that
consumers on older toolchains should use the MSRV-aware resolver. Residual
risk: a 1.85 consumer on `resolver = "2"` without a lock file can still pick
`uuid` 1.29 (1.89), `idna_adapter` 1.2.2 (1.86), or `icu_*` 2.4 (1.88). Not
tested: a real consumer crate built that way; do that before relying on the
README note.

**14 — applied.** `libtea_core::escape_control_chars` (new public function)
turns C0, DEL, C1, and bidi override/isolate characters into `\uXXXX`. It is
applied when `libtea-client` builds `ClientError::Api` (code and message),
`Token`, `Unresolved` (both fields), `Json`, and `Transport`, and in `Display`
of `ModelError::{Json, Invalid}` (which quote server values). The stored text
is changed only if it contained such characters. Tests:
`crates/tea-client/tests/error_text.rs`, `crates/tea-core/tests/escape_control_chars.rs`.
Not covered: `ClientError::Policy` (host and scheme come from a parsed URL, which
rejects control characters) and any text a caller takes from a model field.

### Superseded on 2026-11-08: MSRV raised to 1.89 before the first publish

The maintainer measured `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=allow cargo
update`: the highest `rust-version` among resolved packages is 1.89.0 (`uuid`
1.29.1); `icu_*` 2.4.0 and `proptest` 1.12.0 need 1.88; `idna_adapter` 1.2.2
needs 1.86. Decision: raise the MSRV from 1.85 to 1.89 and stop capping
dependencies. This is option 2 of item 13 taken to its end, and it removes the
residual risk recorded under "Resolution of items 13 and 14" (a Rust 1.85
consumer on `resolver = "2"` can no longer be served, by design). MSRV policy:
latest stable minus about four releases, raised only in minor releases, each
raise listed in the CHANGELOG. Statements above that mention 1.85 describe the
state at the time of each experiment.
