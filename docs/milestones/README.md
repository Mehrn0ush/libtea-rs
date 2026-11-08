# Milestones to 1.0.0

Derived from `docs/roadmap-gap-analysis.md`. Each file lists work items with the reason, testable acceptance criteria, size, dependencies, and the 1.0.0 gate it closes. `gh-issues.sh` holds `gh issue create` commands for every item; it has **not** been run.

Where this order differs from the first suggestion (CLI/lock/Action → purl+discovery+cache → CLE+VEX+adapters → signatures → interop+fuzz+review): the first interop pass and the signature ADR move to 0.2, because their results decide tolerance rules and signatures have the longest lead time; fuzzing in CI and Dependabot also start at 0.2.

| Milestone | Theme | Items |
| --- | --- | --- |
| [0.1](0.1.md) | first publish | 6 |
| [0.2](0.2.md) | compatibility and API decisions, first interop pass, operations basics | 14 |
| [0.3](0.3.md) | CLI for CI | 11 |
| [0.4](0.4.md) | purl, cache, offline mode, allowlist | 6 |
| [0.5](0.5.md) | CLE evaluation and adapters | 4 |
| [0.6](0.6.md) | signature verification | 5 |
| [0.7](0.7.md) | hardening and external review | 6 |
| [1.0](1.0.md) | gates and decision record | 6 |

Several items listed under 0.2–0.4 (for example 0.2-9, the streaming download, JSON and exit-code items, the lock file, `sync`/`diff`, the cache and offline mode) are already in 0.1.0; their checkboxes in each file say which. Decision record "1.0.0 versus upstream TEA beta": end of [1.0.md](1.0.md).
