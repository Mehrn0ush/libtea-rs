# Candidate scope review (2026-11-05)

Review of work added after the pre-existing staged release candidate.

**Status:** recommendations for owner approval. Nothing restaged in this pass.

## Keep in 0.1.0 candidate

| Item | Why |
| --- | --- |
| CLI `TEA_API_KEY_*` + whitespace-preserving resolution + tests | Release hygiene improvement (safer than argv secrets; not a TEA protocol requirement) |
| Client-wide token-refresh coalescing + deterministic test | Reliability improvement under concurrent `401` (documented tradeoff; not a TEA protocol requirement) |
| `consumer_surface` mock tests | Close known Consumer path gaps without live deps |
| Checksum algorithm / multi / incomplete unit tests | Non-breaking; strengthens integrity claims already made |
| Packaging metadata, CHANGELOG, interop/security/README updates | crates.io + honesty |
| CI unpublished-crate check; `.gitignore` for local SBOM | Distribution hygiene |

## Optional CLI conveniences (not library publication requirements)

| Item | Why |
| --- | --- |
| `collections` command | Thin reuse of public `Service` APIs; caller-managed pagination |
| `--json` on discover/spec/parse-tei/collections/product-release | Script-friendly stdout |

Keep these in the candidate only if you accept the added CLI surface. They are not required to publish `cdx-tea-core` / `cdx-tea-client`.

## Explicitly out of this release candidate

| Item | Why |
| --- | --- |
| `docs/signature-verification-design.md` | Preserve locally for later work; do not stage for 0.1.0 |
| Auto-paginator | Caller-managed pagination retained |
| Publishing CLI / conformance | Still `publish = false` |
| `vulnetix-sbom.cdx.json` | Local download artifact; gitignored; do not commit |

## Clarifications

- Environment credentials and token-request coalescing are **engineering improvements**, not mandatory TEA 1.0.0 protocol obligations.
- Checksum verification remains content integrity only; it is not publisher authenticity.
