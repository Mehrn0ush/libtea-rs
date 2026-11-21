# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |
| unreleased `main` | Best effort |

## Reporting a vulnerability

Report security issues privately to the repository owner via GitHub Security
Advisories for [Mehrn0ush/libtea-rs](https://github.com/Mehrn0ush/libtea-rs), or
by contacting the maintainer account that owns the repository.

Please include: affected crate/version, reproduction steps, and impact.
Do **not** open a public issue for unfixed vulnerabilities.

## Non-goals that affect security expectations

- Checksum verification is **not** publisher authenticity.
- Signature bytes may be downloaded. They are verified only as raw Ed25519 or
  ECDSA P-256 signatures under public keys the caller supplies; no key is
  trusted by default or fetched, and OpenPGP, CMS and Sigstore are not verified
  (see `docs/design/signature-verification.md`).
- TLS certificate verification cannot be disabled in `libtea-client`.
