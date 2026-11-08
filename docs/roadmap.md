# Roadmap and deliberate non-goals

Milestones are planning targets, not commitments.

## Deliberate non-goals for 0.1.0

| Item | Rationale |
| --- | --- |
| Publisher API | Separate OpenAPI; out of consumer scope |
| Official CycloneDX / ECMA certification | Independent implementation; offline suite is smoke only |
| Built-in signature authenticity | No trust policy in TEA OpenAPI; see `docs/design/signature-verification.md` |
| Universal server compatibility | Evidence is named deployments only |

## Targeted milestones

| Item | Target | Approach |
| --- | --- | --- |
| HTTPS SVCB lookup | 0.2+ | Optional; discovery text allows it when supported. Prefer document until resolver story is clear. |
| Persistent HTTP cache (`Cache-Control`) | 0.2+ | Non-goal for default client; callers may send `If-None-Match`. A cache layer would need eviction and security review. |
| OAuth beyond API-key `client_credentials` | 0.2–0.3 | Documented non-goal until a concrete profile is required by interoperability. |
| Full ECMA-428 CLE fetch/validate | 0.3+ | Client retrieves TEA CLE projection only. |
| `no_std` for `libtea-core` | undecided | Would require splitting alloc/serde assumptions; not started. |
| Multi-platform CLI release binaries | when CLI publishes | GitHub Actions matrix / `cross`; not required for library crates.io publish. |
| Signature verification (caller trust) | 0.2 / 0.3 | Extension point in 0.1.0; crypto later. |
| Pagination stream helpers | 0.1.0 | Shipped as opt-in `follow_pages`; `ListParams` remains primary. |

## Spec re-pin

See `docs/re-pinning.md` and `spec/PINNED.md`.
