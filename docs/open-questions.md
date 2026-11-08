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

## F1: streaming download, JSON output, exit codes (2026-11-08)

**15 — Range resume: not supported.** Options: (a) do not support it; (b)
`Range` plus `If-Range` on identity-coded bodies only. Recommendation and
current state: (a). A `Range` applies to the encoded representation but the
checksums cover decoded bytes; a partial file would have to be re-read to
rebuild the hash state; TEA 1.0.0 defines no range semantics for artifact
downloads; and a resumed body from a changed object would be hashed as one
file. Revisit if a server publishes `Accept-Ranges` for artifacts and a user
needs it. The milestone item 0.3-6 stays open.

**16 — `download --json` goes to stdout and needs `--output`.** The milestone
text proposed stderr or `--status-file` with stdout staying the bytes. With
`--output` stdout is free, so the JSON object goes there; without `--output`
`--json` is a usage error. Options: keep this, or add `--status-file` later.
Recommendation: keep this.

**17 — error objects go to stderr.** Keeping stdout empty on failure means a
consumer piping stdout to a JSON parser never sees a half document. The cost is
that a consumer must read stderr for the reason. Recommendation: keep.

**18 — exit code 7 is reserved, not emitted.** F1 asks for a "changes detected"
code, but no F1 command compares anything. It is defined now so the number is
fixed before `sync --check` and `diff` exist. Network failure and policy refusal
are not separated from code 1 yet (the milestone lists them); recommendation:
split them only when a pipeline needs it.

**19 — stdout mode buffers.** `download` without `--output` keeps the body in
memory (up to `max_artifact_bytes`) and prints it after the check, because
stdout cannot take bytes back. Recommendation: keep; use `--output` for large
artifacts.

## F2: lock file, sync, diff (2026-11-08)

**20 — `sync --check` downloads everything.** Size and integrity are properties
of the bytes, so a check that only compared metadata would miss a changed body
behind unchanged metadata. Cost: a full download per check. Option: a
`--metadata-only` mode that compares declared checksums and revisions without
fetching. Recommendation: add it only when a pipeline complains about time;
keep the full check as the default.

**21 — no ETag or conditional requests in the lock.** The plan listed ETags;
the lock leaves them out because a validator is not an integrity statement and
adding it would make every ETag rotation a lock diff. Conditional requests
would need it. Recommendation: not now; revisit with `--metadata-only`.

**22 — an unchanged lock is left alone, including its old `fetchedAt`.** This
keeps re-runs out of version control. Cost: `fetchedAt` is the time of the last
change, not of the last successful check. Recommendation: keep; `--check`
exit 0 is the evidence of a recent check.

**23 — `NotDeclared` is allowed into a lock.** The format has no checksum, so
nothing was verified; the lock says so (`integrity: NotDeclared`). Option: a
`--require-checksums` for `sync`, as `download` has. Recommendation: add it
before the GitHub Action (F4) relies on `sync`; one product release in a
pipeline will usually want it.

**24 — unknown fields in a lock are an error.** The lock is our file and is
version-gated; ignoring a field a newer writer added would lose it on the next
write. Cost: an older `libtea` cannot read a newer lock even for `diff`.
Recommendation: keep; bump `schemaVersion` for any addition.

**25 — a lock is unsigned.** Anyone who can edit it can change it. F8 will add
signer identity and trust-policy ID once signature verification exists.
Nothing in the format claims authenticity today.

**26 — one product release per lock.** A TEI that resolves to several needs
`--product-release`. Recommendation: one lock per release; a multi-release
manifest is a later schema.

## F3: release workflow (2026-11-08)

**27 — placeholders for pins.** No network was available, so the SHAs of
`actions/upload-artifact`, `actions/download-artifact`,
`actions/attest-build-provenance` and the `cargo-cyclonedx` version are
placeholders that fail at job setup (`docs/release-pins.md`). Options: ship with
tag pins and list them as debt (rejected: F3 requires SHA pins), or placeholders
(chosen). A gated test fails until they are filled in.

**28 — artifact hand-off between jobs.** Build jobs run on different
operating systems, so artifacts must pass through the Actions artifact store;
the attestation then covers the files as the `attest` job downloaded them, not
as each build job produced them. A tampered artifact between the jobs would be
attested. Mitigations: short retention, no write tokens in build jobs. Option:
attest inside each build job (needs `id-token: write` and `attestations: write`
on every build job, widening their authority). Recommendation: keep the
single `attest` job; revisit if build-job isolation matters more than token
scope.

**29 — macOS x86_64 is cross-compiled** on an arm64 runner and its smoke test is
skipped (`smoke: false`). Option: add an Intel runner when one is available.
Recommendation: keep; say so in `docs/installing.md` (done).

**30 — musl builds unverified.** Linking `ring` against musl with `musl-gcc`
(`CC_<target>` and the target linker set in the build step) is the standard
recipe but was not run. The musl smoke test is the first check. Fallback if it
fails: build musl with `cargo zigbuild`, which adds a pinned tool.

**31 — no code signing or notarization.** The release has provenance and
checksums, not Apple notarization or Authenticode. Recommendation: not in 0.3;
the docs say so.

**32 — runner labels.** `ubuntu-24.04-arm` and `macos-latest` are GitHub-hosted
labels as documented at the time of writing and were not checked. A label that
does not exist leaves the job queued, which a dry run shows at once.

## F4: GitHub Action (2026-11-08)

**33 — the action installs from a release that does not exist yet.** It can
only be exercised end to end after the first release (F3). Until then the
scripts are tested against stubs. Recommendation: make the F3 dry run the first
real test of the action, by pointing `repository` at the dry-run tag's release
(`version: 0.1.0-dryrun.1` works because the release is a draft prerelease only
if you publish it; a draft is not downloadable anonymously, so publish the
dry-run release as a prerelease, test, then delete it).

**34 — `actions/cache` is the action's one third-party dependency**, a
placeholder pin (`docs/release-pins.md`). Option: drop caching and always
download (one archive plus a one-line SHA256SUMS). Recommendation: keep the
cache, it is switchable with `cache: "false"`, and the archive is re-verified
on every run.

**35 — attestation `auto` mode warns and continues without `gh`.** On a
hosted runner `gh` is always present, so this only affects self-hosted runners.
Option: make `required` the default. Recommendation: keep `auto` until the
first release proves `gh attestation verify` works with the flags used (they
are unverified); then make `required` the default.

**36 — no `latest` version.** Pinning is mandatory so a compromised or buggy
newer release cannot silently reach a pipeline. Cost: a manual bump.
Recommendation: keep.

**37 — `extra-args` is a power input.** It can pass any `libtea` flag,
including `--output` to a path of the caller's choosing and `--allow-origin`.
That is the caller's workflow, in the caller's repository, so it is not a
privilege boundary, but a workflow that builds `extra-args` from untrusted text
(a pull-request title, a branch name) would let that text choose flags. The
docs say to keep it static; the action cannot tell the difference.

**38 — the `sync --check` default is not `--frozen`-style for artifacts.** It
re-downloads every artifact (open question 20). Recommendation: unchanged.

## F6: persistent cache, offline mode, Retry-After (2026-11-08)

**39 — always revalidate; `max-age` is not used.** Skipping the request while an
entry is "fresh" would save calls but could serve a stale `collection/latest`.
Recommendation: keep; offline mode is the explicit way to use the cache without
asking.

**40 — `private` is stored only with `--cache-authenticated`.** A per-user cache
may keep `private` responses; the conservative reading of F6 is chosen. Option:
store `private` for anonymous requests. Recommendation: keep until a server's
behavior shows the cost.

**41 — the cache cannot detect forged metadata.** Artifact bytes are re-checked
against declared checksums, but the checksums come from metadata that may itself
be cached. Defence is the private directory, not the format. Option: a keyed MAC
on entries (needs a secret to store, which the cache must not hold) or signature
verification (F8). Recommendation: keep, and say so (done in `docs/cache.md`).

**42 — age counts from the last revalidation.** An entry that keeps getting
`304`s never expires. Option: count from first store to force a periodic full
refetch. Recommendation: keep; `cache clear` exists.

**43 — no `fsync`.** A crash can lose entries. Recommendation: keep; it is a cache.

**44 — Windows has no mode checks.** The `0700`/`0600` rules are Unix-only. A
Windows directory check would need ACL inspection. Recommendation: document
(done); revisit if Windows users report shared profiles.

**45 — discovery documents are cached too**, because they go through the same
transport. That makes `sync --offline` possible and also means a stale
`.well-known/tea` is served offline. Recommendation: keep.

**46 — a streamed artifact is copied into the cache while it downloads**, which
needs space for two copies of a large artifact. Option: hard-link or rename the
destination into the cache. Recommendation: keep the copy; a link would tie the
cache entry to a file the caller may edit.

**47 — a final `503` is now `ClientError::Throttled`**, not
`Transport("HTTP 503")`. This is a visible change for callers that matched the
old text; the milestone asked for a typed error. Recommendation: keep, noted in
the CHANGELOG.

**48 — an offline miss exits 1**, not a dedicated code. Recommendation: keep
unless a pipeline needs to tell "not cached" from other failures.

**49 — the cache is not shared between `Client`s on the same directory for
`prune`.** Each client prunes on its own writes; two clients can briefly exceed
the total. Recommendation: keep; the limit is a ceiling on a quiet directory,
not a quota.
