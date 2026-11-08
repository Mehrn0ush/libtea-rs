# Signature verification — design (libtea-rs)

**Status for 0.1.0:** design + extension point only.  
**Ship rule:** no built-in verifier returns “Verified”.

Checksum `IntegrityOutcome::Verified` means declared digests matched **decoded**
artifact bytes. It does **not** mean a publisher signed the bytes.

## Threat model (consumer)

| Asset | Threat | Checksums alone | Crypto signature (future) |
| --- | --- | --- | --- |
| Artifact bytes in transit/at rest as published by the TEA host | Bit flips / wrong object | Detected if digests are present and match | Same host can publish matching fake digests |
| Publisher identity | Impersonation by TEA host or CDN | **Not** addressed | Requires trust anchors **not** solely from that host |
| Supply-chain attestation (Sigstore, etc.) | Forged provenance | **Not** addressed | Needs log/policy evaluation |

Verifying a signature with keys fetched only from the same untrusted host that
served the artifact would be misleading. Trust anchors must be caller-supplied
or obtained from an independent trust root.

## What “Verified” would mean (future)

Only after **all** of:

1. Signature bytes parse under a supported format.
2. Cryptographic validation succeeds.
3. Signer identity binds to a **caller-configured** trust policy.
4. Optional: transparency-log / freshness checks for Sigstore-like formats.

Never map “bytes downloaded” or “checksum matched” to cryptographic Verified.

## Trust-anchor sourcing

- TEA OpenAPI at the pinned commit does **not** define `/trust-anchors/`.
  Informative `signatures/signature.md` is outside the standard and is not
  implemented as an API surface here.
- 0.1.0: no automatic trust-anchor fetch.
- Future: PEM/keyring trait, and optionally Sigstore TUF roots — caller opts in.

## Formats to consider (roadmap)

| Format | Milestone | Notes |
| --- | --- | --- |
| Detached raw signature over digest with caller keys | 0.2 (candidate) | Smallest honest MVP |
| CMS / OpenPGP detached | 0.2–0.3 | Only if publishers converge |
| Sigstore bundle | 0.3 | Separate feature; log policy required |

## Failure modes

`SignatureVerifyOutcome`: `Verified` | `Invalid` | `Unsupported` |
`NoTrustAnchor` | `MissingSignature` (see `libtea_client::signature`).

## Extension point (0.1.0)

`libtea_client::SignatureVerifier` receives artifact bytes + signature bytes and
returns a typed outcome. The HTTP client does **not** call it automatically.
Test doubles must not return `Verified` unless simulating a caller policy in
tests.

## Staged roadmap

- **0.1.0:** download signature bytes; checksum integrity; trait + design doc.
- **0.2.0:** optional verifier crate/feature with caller-supplied keys; no default
  trust.
- **0.3.0:** optional Sigstore path behind an explicit feature and documented
  policy inputs.
