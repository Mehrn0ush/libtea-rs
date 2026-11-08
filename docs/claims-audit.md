# Overclaim sweep (verification pass)

Case-insensitive grep for: verified, verify, authentic, trusted, trust,
compatible, compatibility, certified, certification, conformant, secure,
guarantee, always, never — across rustdoc, README, docs, CHANGELOG, CLI,
conformance strings, and comments (excluding `fuzz/crates/` mirror noise).

## Changed hits

| File:approx | Before (summary) | After | Reason |
| --- | --- | --- | --- |
| `docs/auth.md` | Bound stated without empty-cache / no mixed-retry conditions | Bound scoped to empty/expired cache, same origin, first exchange succeeds; mutex client-wide; not lifetime exactly-once | Match N=32 test + transport |
| `crates/tea-client/src/lib.rs` crate docs | Short single-flight sentence | Same conditions as `docs/auth.md` | Consistency |
| `crates/tea-client/src/transport.rs` `token_refresh` docs | Broad “guarantee” of 1 exchange | Bound + conditions; “guarantee” wording softened | Match test |
| `crates/tea-client/src/service.rs` `DownloadedArtifact::integrity` | “every declared checksum matched” | “…matched **decoded** artifact bytes — not publisher authenticity” | Qualify Verified |
| `crates/tea-client/src/service.rs` `download_format` | “checksums are verified over…” | “checked… (integrity of decoded bytes, not authenticity)” | Avoid “verified” alone |
| `crates/tea-cli/src/main.rs` download stderr | `integrity verified\t…` | Adds `(decoded-byte checksums only; not publisher authenticity)` | Qualify CLI claim |
| `crates/tea-cli/tests/download.rs` | assert contains `integrity verified` | Also asserts authenticity disclaimer present | Keep test honest |
| `README.md` Vulnetix download note | “prints `integrity verified`” | Mentions decoded-byte / not authenticity | Match CLI |
| `scripts/interop-probe.sh` | Stored `verified` / raw CLI out | `checksum_match_decoded_bytes`; sanitize secrets; chmod 600 | Safety + honesty |
| `CHANGELOG.md` | Broad token bound | Empty-cache conditions; experimental markers | Accuracy |
| `crates/tea-client/src/page_stream.rs` / `signature.rs` | No experimental note | **Experimental: may change in a 0.x minor** | T7 |

## Unchanged hits (justification)

| Hit | Justification |
| --- | --- |
| `IntegrityOutcome::Verified` enum / checksum code | Domain term; rustdoc/README already qualify ≠ authenticity |
| `SignatureVerifyOutcome::Verified` / “no verifier returns Verified” | Negative claim; tested |
| `certificate verification` / TLS trust anchors | Literal TLS; “cannot be disabled” is accurate |
| Matrix / interop “not certification / not compatibility” | Honesty posture |
| `conformance` crate / CLI command name | Smoke suite; report sets `is_certification: false` |
| Auth tests / “authentication required” | Protocol wording |
| DestinationPolicy / “untrusted TEA responses” Limits docs | Size bounds on untrusted input |
| `always restore` env in CLI tests | Test hygiene, not product claim |
| Design docs discussing trust anchors / authenticity | Threat-model language; no shipped crypto claim |
| `docs/security.md` content-coding “Unknown… are errors” | Behaviour under test |

## Residual risk

Grep volume is large (~500+ hits). Future edits that print `Verified` without the decoded-bytes qualifier should be rejected in review.

## 2026-11-07 audit sweep

Same terms, re-run over `README.md`, crate READMEs, `CHANGELOG.md`,
`SECURITY.md`, `docs/`, rustdoc in `crates/*/src`, CLI strings, and scripts
(excluding `spec/vendor/` and test code).

### Changed

| File | Before | After | Reason |
| --- | --- | --- | --- |
| `crates/tea-core/src/checksum.rs` `IntegrityOutcome::Verified` rustdoc | "Every declared checksum was computed and matched." | Adds "over the decoded artifact bytes … content integrity only, not publisher authenticity" | Unqualified `Verified` on the most-read enum |
| `crates/tea-client/src/error.rs` `ClientError::Integrity` rustdoc | "Declared checksums were not fully verified." | "did not all match the decoded artifact bytes, or at least one could not be computed" | Says what was compared |
| `CHANGELOG.md` Security | "never implied authenticity" | "matches the decoded artifact bytes; it does not imply publisher authenticity" | Drops the absolute; adds the decoded-bytes scope |
| `crates/tea-cli/src/main.rs` `conformance` help | "offline conformance checks" | "offline smoke checks … (not a certification)" | Matches `is_certification: false` |
| `README.md` heading | "Verify (CLI and examples)" | "Try it (CLI and examples)" | The section runs commands; it does not verify anything |
| `docs/auth.md`, `crates/tea-client/src/lib.rs`, `transport.rs`, `README.md`, `docs/security.md`, `docs/interoperability.md` | "client-wide mutex"; bound conditions that contradicted the exception list | Per-origin locks; bound stated with its two conditions; exceptions are a just-stored token already expired, a later call after a failure, cancellation, rejected bearer, other origin | Behavior changed (AUD-03); old text was self-contradictory |
| `scripts/interop-probe.sh` | Pass decided by grepping stderr for `integrity verified` | Pass decided by exit status of `download --require-checksums` | Prose coupling removed |
| `docs/release-checklist.md` | "CI green … plus … audit" | `cargo audit` is `continue-on-error`; `cargo deny` gates advisories | The old text claimed a gate that does not exist |
| `src/page_stream.rs` `PageFollowLimits::max_items` | "Maximum number of items to yield" | Stop after the page on which the count reaches it; may exceed by one page | The old text overclaimed the bound |

### Kept, with reason

| Hit | Reason |
| --- | --- |
| `docs/conformance-gap-analysis.md` "Verified" status column | Test-status label defined at the top of that file, not an integrity or certification claim |
| `docs/audit-report.md` line 51 "never verifies artifact bytes" | Historical finding from 2026-10-10, fixed by TEA-AUD-005 in the same file |
| `SECURITY.md` / README "certificate verification cannot be disabled" | True: no `danger_accept_invalid_certs` call exists; `add_root_pem` only adds anchors |
| "always `false`" for `is_certification` | Literal; asserted by `report_is_not_a_certification_and_names_the_pinned_commit` |
| `SignatureVerifyOutcome::Verified` | Exists for caller-supplied verifiers; no built-in code path returns it (grep: only the enum definition and a negative test) |
