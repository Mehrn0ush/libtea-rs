# Security Policy

## Supported versions

| Crate | Supported | Fixes ship in |
| --- | --- | --- |
| `libtea-core` | latest release (0.2.x) | the next 0.2.x release |
| `libtea-client` | latest release (0.2.x) | the next 0.2.x release |
| `libtea-cli` (`libtea` binaries, GitHub Action) | latest release (0.2.x) | the next 0.2.x release |
| unreleased `main` | best effort | |

**Support period: no end date.** The project intends to support its latest
release indefinitely. Security fixes are released for the latest minor line of
each crate; to receive a fix, upgrade to it. Older lines get no backports. If
this ever changes, the end-of-support date will be written here and in the
release notes before it takes effect.

## Reporting a vulnerability

Report privately through GitHub private vulnerability reporting:
<https://github.com/Mehrn0ush/libtea-rs/security/advisories/new>. Do **not**
open a public issue for an unfixed vulnerability.

Please include: affected crate and version, reproduction steps, and impact.

What to expect:

- an acknowledgement within **14 days**;
- an assessment (accepted, or why not) within **30 days**;
- for an accepted vulnerability of medium severity or higher, a fixed release
  within **60 days**, sooner for anything actively exploited;
- a GitHub Security Advisory published with the fixed release, crediting you
  unless you ask not to be named. Advisories for published crates are also
  submitted to the RustSec advisory database.

If a fix needs longer, you will be told why and when to expect it.

## Vulnerabilities in dependencies

`cargo deny` and `cargo audit` run in CI, and Dependabot proposes updates. When
an advisory affects a dependency in a released `Cargo.lock`, the assessment is
recorded in [`vex/libtea.vex.cdx.json`](vex/libtea.vex.cdx.json), a CycloneDX
VEX document: whether libtea is affected, and why not when it is not. A fixed
dependency ships in the next release.

## What each release provides

- a CycloneDX SBOM of the CLI's dependency tree (`libtea-X.Y.Z.cdx.json`), with
  the supplier and the links above;
- `SHA256SUMS` and a GitHub build-provenance attestation for every archive,
  attached to the release as `libtea-X.Y.Z.sigstore.json` and
  `libtea-X.Y.Z.intoto.jsonl` from the first release after 0.2.2; how to
  verify them is in [`docs/installing.md`](docs/installing.md);
- release notes in [`CHANGELOG.md`](CHANGELOG.md).

The security design (assets, trust boundaries, transport and integrity rules)
is in [`docs/security.md`](docs/security.md).

## Non-goals that affect security expectations

- Checksum verification is **not** publisher authenticity.
- Signature bytes may be downloaded. They are verified only as raw Ed25519 or
  ECDSA P-256 signatures under public keys the caller supplies; no key is
  trusted by default or fetched, and OpenPGP, CMS and Sigstore are not verified
  (see `docs/design/signature-verification.md`).
- TLS certificate verification cannot be disabled in `libtea-client`.
