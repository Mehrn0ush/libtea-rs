# Roadmap and deliberate non-goals

Milestones are planning targets, not commitments. This is the first public
release, 0.1.0; everything below is stated against it.

## Deliberate non-goals for 0.1.0

| Item | Rationale |
| --- | --- |
| Publisher API | Separate OpenAPI; out of consumer scope |
| Official CycloneDX / ECMA certification | Independent implementation; the offline suite is a smoke test only |
| Built-in signature authenticity | No trust policy in the TEA OpenAPI; see `docs/design/signature-verification.md` |
| Universal server compatibility | Evidence is named deployments only |
| Publishing `libtea-cli` and `libtea-conformance` to crates.io | Both are `publish = false`; the CLI ships as release binaries and via `cargo install --git` (`docs/release-checklist.md`) |

## In 0.1.0 (experimental surfaces are marked in their docs)

Streaming downloads, `--json` and exit codes, the lock file with `sync` and
`diff`, the opt-in persistent cache and offline mode, `Retry-After` handling,
the release workflow and the GitHub Action. The release workflow and the Action
have been tested against stubs only; they have not run on GitHub
(`docs/release-checklist.md`).

## Later

| Item | Target | Approach |
| --- | --- | --- |
| HTTPS SVCB lookup | 0.2+ | Optional; discovery text allows it when supported. Prefer a document until the resolver story is clear. |
| OAuth beyond API-key `client_credentials` | 0.2–0.3 | Not planned until a concrete profile is required for interoperability. |
| Full ECMA-428 CLE fetch/validate | 0.3+ | The client retrieves the TEA CLE projection only. |
| purl / VERS matching | separate run | Stage 1 supports the `semver` VERS scheme only and returns `Unknown` for others. |
| Signature verification (caller-supplied trust) | separate run | Stage 1: raw detached signatures over decoded artifact bytes, `ring`, no base64 signatures, Sigstore deferred. |
| Folding `libtea-conformance` into the CLI | 0.2 | See `docs/release-checklist.md`. |
| `no_std` for `libtea-core` | undecided | Would require splitting alloc/serde assumptions; not started. |
| Pagination stream helpers | 0.1.0 | Shipped as opt-in `follow_pages`; `ListParams` remains primary. |

## Spec re-pin

See `docs/re-pinning.md` and `spec/PINNED.md`.
