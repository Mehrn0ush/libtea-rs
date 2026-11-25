# Governance

libtea-rs is an independent project. It is not part of CycloneDX, OWASP or
Ecma, and it does not speak for the Transparency Exchange API (TEA) working
group.

## Decision making

The project has one maintainer, [@Mehrn0ush](https://github.com/Mehrn0ush),
who makes the final decision on every change, release and setting. Decisions
are made in the open: a change is proposed as a pull request or an issue, the
reasoning is written there or in a design note under `docs/design/`, and the
maintainer merges or declines it with a reason. Anyone may comment on a pull
request or an issue.

What a change must meet is in [CONTRIBUTING.md](CONTRIBUTING.md) (the gate,
sign-off, tests, one concern per pull request). How releases are cut is in
[docs/release-checklist.md](docs/release-checklist.md).

## Roles

| Role | Who | Responsibilities |
| --- | --- | --- |
| Maintainer | [@Mehrn0ush](https://github.com/Mehrn0ush) | Reviews and merges pull requests; triages issues; handles vulnerability reports ([SECURITY.md](SECURITY.md)); approves the `release` environment, tags releases and publishes them to GitHub Releases and crates.io; owns the repository settings and the crates on crates.io; enforces the [code of conduct](CODE_OF_CONDUCT.md). |
| Contributor | anyone | Opens issues and pull requests that follow [CONTRIBUTING.md](CONTRIBUTING.md), with every commit signed off (Developer Certificate of Origin). |

Automated tools open pull requests too (Dependabot). They are reviewed and
merged like any other contribution.

## Becoming a maintainer

A regular contributor can be invited to become a maintainer by the current
maintainer. A new maintainer gets write access to the repository, a required
reviewer slot on the `release` environment, and co-ownership of the crates on
crates.io, and is added to the table above in a pull request.

## Continuity

Today the maintainer is the only person with admin access to the repository,
ownership of `libtea-core`, `libtea-client` and `libtea-cli` on crates.io, and
approval rights on the `release` environment. If the maintainer became
unavailable, nobody else could merge, release or publish. The source, the
history and every release stay public under the Apache-2.0 license, so the
project can be forked, but it could not continue under these names without that
access. Adding a second maintainer, as above, is how this will be fixed.
